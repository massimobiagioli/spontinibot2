use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use cron::Schedule;
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
}

impl CronScheduler {
    pub fn new(run_poll_secs: u64) -> Self {
        Self { run_poll_secs }
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
                    if let Some(ref config) = config
                        && !config.sources.is_empty()
                    {
                        tracing::info!("run request check: triggering pipeline");
                        if let Err(e) = runner.run_all(&config.sources).await {
                            tracing::error!("run request pipeline execution failed: {e}");
                        }
                    }
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
}
