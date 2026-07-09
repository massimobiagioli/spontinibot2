use backend::router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("backend listening on 0.0.0.0:8080");

    axum::serve(listener, router()).await?;
    Ok(())
}
