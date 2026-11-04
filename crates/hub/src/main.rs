//! Hub binary: reads the environment, builds the dependencies and serves the router.

use std::sync::Arc;

use anima_hub::{clock::SystemClock, config, events::process_log, router, schema::build_schema};

#[derive(Debug, thiserror::Error)]
enum StartError {
    #[error("invalid configuration")]
    Config(#[from] config::Error),
    #[error("invalid log setup")]
    Log(#[from] process_log::Error),
    #[error("hub failed to serve")]
    Serve(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), StartError> {
    let env = config::collect_env(std::env::vars_os())?;
    let config = Arc::new(config::from_env(&env)?);
    process_log::init(&config.log)?;
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    tracing::info!(listen = %config.listen, "hub started");
    let schema = build_schema(config, Arc::new(SystemClock));
    Ok(axum::serve(listener, router(schema)).await?)
}
