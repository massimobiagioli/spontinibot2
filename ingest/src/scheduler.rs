use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use cron::Schedule;
use kb_store::{KbStore, RunRequestStatus};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::config::IngestConfig;
use crate::error::IngestError;
use crate::runner::PipelineRunner;

fn parse_cron(expr: &str) -> Result<Schedule, IngestError> {
    expr.parse::<Schedule>()
        .map_err(|e| IngestError::Cron(e.to_string()))
}

fn next_tick(expr: &str) -> Result<chrono::DateTime<Utc>, IngestError> {
    let schedule = parse_cron(expr)?;
    schedule
        .upcoming(Utc)
        .next()
        .ok_or_else(|| IngestError::Cron("no upcoming tick in schedule".into()))
}

fn sleep_until_next_tick(expr: &str) -> Result<Duration, IngestError> {
    let next = next_tick(expr)?;
    let dur = next - Utc::now();
    let secs = dur.num_seconds().max(1) as u64;
    Ok(Duration::from_secs(secs))
}

pub struct CronScheduler {
    run_poll_secs: u64,
    heartbeat_path: PathBuf,
    kb: KbStore,
}

impl CronScheduler {
    pub fn new(run_poll_secs: u64, heartbeat_path: PathBuf, kb: KbStore) -> Self {
        Self {
            run_poll_secs,
            heartbeat_path,
            kb,
        }
    }

    /// Consumes the oldest pending run request (if any) and runs the pipeline for
    /// it, marking the request done/failed when finished. No-ops when there's
    /// nothing pending — this is what gates pipeline execution behind an actual
    /// `POST /admin/api/ingest/run` instead of running unconditionally on every
    /// tick (see Appendix C of TEST-INGESTION-0001.md for the incident this fixes).
    async fn consume_and_run(&self, config: &Option<IngestConfig>, runner: &Arc<PipelineRunner>) {
        let request = match self.kb.consume_run_request().await {
            Ok(Some(request)) => request,
            Ok(None) => return,
            Err(e) => {
                tracing::error!("failed to poll for pending run request: {e}");
                return;
            }
        };

        let status = match config {
            Some(config) if !config.sources.is_empty() => {
                tracing::info!(
                    "run request {} consumed: triggering pipeline for {} sources",
                    request.id,
                    config.sources.len()
                );
                match runner.run_all(&config.sources).await {
                    Ok(()) => RunRequestStatus::Done,
                    Err(e) => {
                        tracing::error!("run request {} pipeline execution failed: {e}", request.id);
                        RunRequestStatus::Failed
                    }
                }
            }
            _ => {
                tracing::info!(
                    "run request {} consumed but no sources are configured; marking done",
                    request.id
                );
                RunRequestStatus::Done
            }
        };

        if let Err(e) = self.kb.complete_run(request.id, status).await {
            tracing::error!("failed to mark run request {} complete: {e}", request.id);
        }
    }

    /// Touches the heartbeat file so the Docker healthcheck can confirm the
    /// scheduler's poll loop is alive. A write failure is logged, not fatal.
    async fn touch_heartbeat(&self) {
        let now = Utc::now().to_rfc3339();
        if let Err(e) = std::fs::write(&self.heartbeat_path, now) {
            tracing::warn!(
                "failed to write heartbeat file {:?}: {e}",
                self.heartbeat_path
            );
        }
    }

    pub async fn run(
        self,
        mut config_rx: watch::Receiver<Option<IngestConfig>>,
        runner: Arc<PipelineRunner>,
        shutdown: CancellationToken,
    ) -> Result<(), IngestError> {
        tracing::info!(
            "scheduler starting with run_poll_secs={}",
            self.run_poll_secs
        );

        let mut run_interval = tokio::time::interval(Duration::from_secs(self.run_poll_secs));
        let mut cron_sleep = Box::pin(tokio::time::sleep(Duration::MAX));

        loop {
            let config = config_rx.borrow_and_update().clone();

            if let Some(ref config) = config
                && config.schedule_enabled
                && !config.sources.is_empty()
            {
                match sleep_until_next_tick(&config.cron_expression) {
                    Ok(dur) => {
                        tracing::debug!("cron sleep until next tick in {}s", dur.as_secs());
                        cron_sleep.as_mut().reset(tokio::time::Instant::now() + dur);
                    }
                    Err(e) => {
                        tracing::warn!("failed to compute next cron tick: {e}");
                        cron_sleep
                            .as_mut()
                            .reset(tokio::time::Instant::now() + Duration::from_secs(3600));
                    }
                }
            } else {
                cron_sleep
                    .as_mut()
                    .reset(tokio::time::Instant::now() + Duration::from_secs(86400));
            }

            let should_stop = tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("scheduler received shutdown signal");
                    true
                }
                _ = config_rx.changed() => {
                    tracing::debug!("configuration changed, recomputing schedule");
                    false
                }
                _ = run_interval.tick() => {
                    self.touch_heartbeat().await;
                    self.consume_and_run(&config, &runner).await;
                    false
                }
                _ = cron_sleep.as_mut() => {
                    if let Some(ref config) = config
                        && config.schedule_enabled
                        && !config.sources.is_empty()
                    {
                        tracing::info!(
                            "cron tick fired, running pipeline for {} sources",
                            config.sources.len()
                        );
                        if let Err(e) = runner.run_all(&config.sources).await {
                            tracing::error!("cron pipeline execution failed: {e}");
                        }
                    }
                    false
                }
            };

            if should_stop {
                return Err(IngestError::Shutdown);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    async fn temp_kb() -> KbStore {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ingest_scheduler_test_{}_{}.db",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    #[test]
    fn should_parse_valid_cron_expression() {
        let schedule = parse_cron("0 0 */4 * * * *");
        assert!(schedule.is_ok());
    }

    #[test]
    fn should_reject_invalid_cron_expression() {
        let schedule = parse_cron("not-a-cron");
        assert!(schedule.is_err());
    }

    #[test]
    fn should_compute_next_tick_in_future() {
        let dt = next_tick("0 0 */4 * * * *").expect("parse failed");
        assert!(dt > Utc::now(), "next tick should be in the future");
    }

    #[tokio::test]
    async fn should_write_heartbeat_file_with_recent_timestamp() {
        let path = std::env::temp_dir().join(format!(
            "ingest_heartbeat_test_{}_{}",
            std::process::id(),
            "ok"
        ));
        let scheduler = CronScheduler::new(10, path.clone(), temp_kb().await);

        scheduler.touch_heartbeat().await;

        let contents = std::fs::read_to_string(&path).expect("heartbeat file should exist");
        assert!(!contents.is_empty());
        let written_at: chrono::DateTime<Utc> = contents
            .parse()
            .expect("heartbeat content should be RFC3339");
        assert!(Utc::now() - written_at < chrono::Duration::seconds(5));

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_not_panic_when_heartbeat_write_fails() {
        let scheduler = CronScheduler::new(
            10,
            PathBuf::from("/nonexistent-dir-spontini-heartbeat-test/heartbeat"),
            temp_kb().await,
        );

        scheduler.touch_heartbeat().await;
    }

    #[tokio::test]
    async fn should_noop_when_no_run_request_is_pending() {
        let kb = temp_kb().await;
        let scheduler = CronScheduler::new(10, std::env::temp_dir().join("unused-heartbeat"), kb);
        let pipeline = crate::runner::create_pipeline(
            "test-agent".into(),
            "http://127.0.0.1:1".into(),
            512,
            64,
            temp_kb().await,
        )
        .expect("failed to create pipeline");
        let runner = Arc::new(PipelineRunner::new(pipeline));

        let config = Some(IngestConfig {
            cron_expression: String::new(),
            schedule_enabled: false,
            sections: vec!["storia".into()],
            sources: vec![crate::config::IngestSource {
                section: "storia".into(),
                url: "https://example.com/should-not-be-fetched".into(),
            }],
        });

        // No run request was ever created, so this must be a no-op: it must not
        // call runner.run_all (which would try to reach the URL above).
        scheduler.consume_and_run(&config, &runner).await;
    }

    #[tokio::test]
    async fn should_mark_run_request_done_when_no_sources_configured() {
        let kb = temp_kb().await;
        let request = kb.request_run().await.expect("request_run failed");

        let scheduler = CronScheduler::new(10, std::env::temp_dir().join("unused-heartbeat"), kb);
        let pipeline = crate::runner::create_pipeline(
            "test-agent".into(),
            "http://127.0.0.1:1".into(),
            512,
            64,
            temp_kb().await,
        )
        .expect("failed to create pipeline");
        let runner = Arc::new(PipelineRunner::new(pipeline));

        scheduler.consume_and_run(&None, &runner).await;

        let fetched = scheduler
            .kb
            .get_run_request(request.id)
            .await
            .expect("get_run_request failed")
            .expect("run request should still exist");
        assert_eq!(fetched.status, RunRequestStatus::Done);
    }
}
