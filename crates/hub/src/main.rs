//! Hub binary: reads the listen address and serves the router.

use std::net::SocketAddr;

const ADDR_VAR: &str = "ANIMA_HUB_ADDR";
const DEFAULT_ADDR: &str = "127.0.0.1:8080";

#[derive(Debug)]
enum StartError {
    Addr {
        value: String,
        source: std::net::AddrParseError,
    },
    Serve(std::io::Error),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Addr { value, source } => {
                write!(f, "{ADDR_VAR}={value:?} is not a socket address: {source}")
            }
            Self::Serve(source) => write!(f, "hub failed to serve: {source}"),
        }
    }
}

impl std::error::Error for StartError {}

fn listen_addr() -> Result<SocketAddr, StartError> {
    let value = std::env::var(ADDR_VAR).unwrap_or_else(|_| DEFAULT_ADDR.to_owned());
    value
        .parse()
        .map_err(|source| StartError::Addr { value, source })
}

#[tokio::main]
async fn main() -> Result<(), StartError> {
    let listener = tokio::net::TcpListener::bind(listen_addr()?)
        .await
        .map_err(StartError::Serve)?;
    axum::serve(listener, anima_hub::router())
        .await
        .map_err(StartError::Serve)
}
