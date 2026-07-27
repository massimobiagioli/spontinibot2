use backend::auth::credential::OperatorCredential;
use backend::config::Config;
use backend::router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let config = Config::from_env();

    if let (Some(username), Some(password)) = (&config.operator_username, &config.operator_password)
    {
        match OperatorCredential::ensure_from_env(
            &config.operator_credential_path,
            username,
            password,
        ) {
            Ok(true) => tracing::info!(
                "operator credential created from env vars at {}",
                config.operator_credential_path
            ),
            Ok(false) => {
                tracing::info!("operator credential file already exists, env vars ignored")
            }
            Err(e) => tracing::warn!("failed to create operator credential from env vars: {e:?}"),
        }
    }

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("backend listening on 0.0.0.0:8080");

    axum::serve(listener, router().await).await?;
    Ok(())
}
