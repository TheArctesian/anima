//! Hub binary: reads the environment, builds the dependencies and serves the router.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::sync::Arc;

use anima_hub::{clock::SystemClock, config, router, schema::build_schema};

#[derive(Debug, thiserror::Error)]
enum StartError {
    #[error("environment variable {name:?} is not valid unicode")]
    EnvNotUnicode { name: String },
    #[error("invalid configuration")]
    Config(#[from] config::Error),
    #[error("hub failed to serve")]
    Serve(#[from] std::io::Error),
}

/// Collect the process environment; non-unicode `ANIMA_*` variables are an error, others are skipped.
fn process_env() -> Result<BTreeMap<String, String>, StartError> {
    let mut env = BTreeMap::new();
    for (name, value) in std::env::vars_os() {
        match (name.into_string(), value.into_string()) {
            (Ok(name), Ok(value)) => {
                env.insert(name, value);
            }
            (Ok(name), Err(_)) if name.starts_with("ANIMA_") => {
                return Err(StartError::EnvNotUnicode { name });
            }
            (Err(name), _) if is_anima(&name) => {
                return Err(StartError::EnvNotUnicode {
                    name: name.to_string_lossy().into_owned(),
                });
            }
            _ => {}
        }
    }
    Ok(env)
}

fn is_anima(name: &OsString) -> bool {
    name.as_encoded_bytes().starts_with(b"ANIMA_")
}

#[tokio::main]
async fn main() -> Result<(), StartError> {
    let config = Arc::new(config::from_env(&process_env()?)?);
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let schema = build_schema(config, Arc::new(SystemClock));
    Ok(axum::serve(listener, router(schema)).await?)
}
