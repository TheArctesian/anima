//! Hub binary: reads the listen address and serves the router.

use std::env::VarError;
use std::net::{AddrParseError, SocketAddr};

const ADDR_VAR: &str = "ANIMA_HUB_ADDR";
const DEFAULT_ADDR: &str = "127.0.0.1:8080";

#[derive(Debug, thiserror::Error)]
enum StartError {
    #[error("{ADDR_VAR} is not valid unicode")]
    AddrNotUnicode,
    #[error("{ADDR_VAR}={value:?} is not a socket address")]
    AddrInvalid {
        value: String,
        #[source]
        source: AddrParseError,
    },
    #[error("hub failed to serve")]
    Serve(#[from] std::io::Error),
}

fn listen_addr() -> Result<SocketAddr, StartError> {
    let value = match std::env::var(ADDR_VAR) {
        Ok(value) => value,
        Err(VarError::NotPresent) => DEFAULT_ADDR.to_owned(),
        Err(VarError::NotUnicode(_)) => return Err(StartError::AddrNotUnicode),
    };
    value
        .parse()
        .map_err(|source| StartError::AddrInvalid { value, source })
}

#[tokio::main]
async fn main() -> Result<(), StartError> {
    let listener = tokio::net::TcpListener::bind(listen_addr()?).await?;
    Ok(axum::serve(listener, anima_hub::router()).await?)
}
