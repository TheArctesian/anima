//! TCP probe: the connect time is the response time.

use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use tokio::net::TcpStream;

use super::{
    ProbeEnv,
    outcome::{Failure, ProbeResult},
    resolve::allowed_addresses,
};

/// Connects to the first allowed address that accepts; returns the stream, the address and
/// the connect time.
pub(super) async fn connect(
    host: &str,
    port: u16,
    env: &ProbeEnv,
) -> Result<(TcpStream, IpAddr, Duration), Failure> {
    let mut last = Failure::Unresolved;
    for ip in allowed_addresses(host, None, env).await? {
        let started = Instant::now();
        match TcpStream::connect(SocketAddr::new(ip, port)).await {
            Ok(stream) => return Ok((stream, ip, started.elapsed())),
            Err(error) => last = error.into(),
        }
    }
    Err(last)
}

pub(super) async fn probe(host: &str, port: u16, env: &ProbeEnv) -> ProbeResult {
    match connect(host, port, env).await {
        Ok((_, ip, elapsed)) => ProbeResult::up(elapsed).via(ip),
        Err(failure) => ProbeResult::failed(&failure),
    }
}
