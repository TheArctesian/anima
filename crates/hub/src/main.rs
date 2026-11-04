//! Hub binary: reads the environment, builds the dependencies and serves the router.

use std::sync::Arc;

use anima_hub::{clock::SystemClock, config, router, schema::build_schema};

#[derive(Debug, thiserror::Error)]
enum StartError {
    #[error("invalid configuration")]
    Config(#[from] config::Error),
    #[error("hub failed to serve")]
    Serve(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), StartError> {
    let env = config::collect_env(std::env::vars_os())?;
    let config = Arc::new(config::from_env(&env)?);
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let schema = build_schema(config, Arc::new(SystemClock));
    Ok(axum::serve(listener, router(schema)).await?)
}
