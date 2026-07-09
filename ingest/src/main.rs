#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    tracing::info!("ingest service started (walking skeleton)");

    tokio::signal::ctrl_c().await?;

    tracing::info!("ingest service stopping");
    Ok(())
}
