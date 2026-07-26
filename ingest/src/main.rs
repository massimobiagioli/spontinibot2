mod config;
mod error;
mod runner;
mod scheduler;

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::config::ConfigLoader;
use crate::config::ConfigWatcher;
use crate::runner::PipelineRunner;
use crate::runner::create_pipeline;
use crate::scheduler::CronScheduler;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let kb_path = env_or("KB_PATH", "/data/kb.db");
    let embedder_base_url = env_or("EMBEDDER_BASE_URL", "http://llama-embed:8080");
    let user_agent = env_or("USER_AGENT", "SpontiniBot/2.0");
    let chunk_size: usize = env_or("CHUNK_SIZE", "512").parse().unwrap_or(512);
    let chunk_overlap: usize = env_or("CHUNK_OVERLAP", "64").parse().unwrap_or(64);
    let config_poll_secs: u64 = env_or("CONFIG_POLL_SECS", "30").parse().unwrap_or(30);
    let run_poll_secs: u64 = env_or("RUN_REQUEST_POLL_SECS", "10").parse().unwrap_or(10);
    let heartbeat_path = env_or("HEARTBEAT_PATH", "/tmp/ingest-heartbeat");

    tracing::info!(
        kb_path = %kb_path,
        embedder_base_url = %embedder_base_url,
        chunk_size = chunk_size,
        chunk_overlap = chunk_overlap,
        config_poll_secs = config_poll_secs,
        run_poll_secs = run_poll_secs,
        heartbeat_path = %heartbeat_path,
        "ingest service starting"
    );

    let shutdown = CancellationToken::new();
    let shutdown_signal = shutdown.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl_c handler");
        tracing::info!("ctrl-c received, initiating graceful shutdown");
        shutdown_signal.cancel();
    });

    // Each long-lived consumer gets its own open KbStore connection, held for the
    // life of the process (never reopened on a timer — see ConfigLoader's doc
    // comment for why that matters).
    let config_kb = kb_store::KbStore::open(&kb_path)
        .await
        .map_err(|e| format!("failed to open kb store (config loader): {e}"))?;
    let scheduler_kb = kb_store::KbStore::open(&kb_path)
        .await
        .map_err(|e| format!("failed to open kb store (scheduler): {e}"))?;
    let pipeline_kb = kb_store::KbStore::open(&kb_path)
        .await
        .map_err(|e| format!("failed to open kb store (pipeline): {e}"))?;

    let loader = ConfigLoader::new(config_kb);
    let watcher = ConfigWatcher::new(loader, config_poll_secs);
    let (config_rx, _watcher_handle) = watcher.run().await;

    let pipeline = create_pipeline(
        user_agent,
        embedder_base_url,
        chunk_size,
        chunk_overlap,
        pipeline_kb,
    )
    .map_err(|e| format!("failed to create pipeline: {e}"))?;

    let runner = Arc::new(PipelineRunner::new(pipeline));

    let scheduler = CronScheduler::new(
        run_poll_secs,
        std::path::PathBuf::from(heartbeat_path),
        scheduler_kb,
    );
    if let Err(e) = scheduler.run(config_rx, runner, shutdown).await {
        tracing::info!("ingest service stopped: {e}");
    } else {
        tracing::info!("ingest service stopped");
    }
    Ok(())
}
