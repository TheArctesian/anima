//! ICMP echo probe over unprivileged datagram sockets. No `ping` process is ever started
//! (CHK-012).

use std::{net::IpAddr, time::Duration};

use surge_ping::{Client, Config, ICMP, PingIdentifier, PingSequence, SurgeError};

use super::{
    ProbeEnv,
    outcome::{Failure, ProbeResult},
    resolve::allowed_addresses,
};
use crate::checks::config::IpVersion;

const PAYLOAD: [u8; 32] = [0x5a; 32];

/// How much later than the check's limit the pinger gives up, so the probe's own limit is the
/// one that ends an echo with no reply.
const PINGER_MARGIN: Duration = Duration::from_millis(500);

pub(super) async fn probe(
    host: &str,
    version: IpVersion,
    limit: Duration,
    env: &ProbeEnv,
) -> ProbeResult {
    match echo(host, version, limit, env).await {
        Ok((ip, elapsed)) => ProbeResult::up(elapsed).via(ip),
        Err(failure) => ProbeResult::failed(&failure),
    }
}

async fn echo(
    host: &str,
    version: IpVersion,
    limit: Duration,
    env: &ProbeEnv,
) -> Result<(IpAddr, Duration), Failure> {
    let ip = *allowed_addresses(host, Some(version), env)
        .await?
        .first()
        .ok_or(Failure::NotPermitted)?;
    let kind = match version {
        IpVersion::V4 => ICMP::V4,
        IpVersion::V6 => ICMP::V6,
    };
    let client = Client::new(&Config::builder().kind(kind).build())
        .map_err(|error| Failure::IcmpSocket(error.kind()))?;
    // The kernel assigns the identifier of a datagram ICMP socket; this one is a placeholder.
    let mut pinger = client.pinger(ip, PingIdentifier(0)).await;
    pinger.timeout(limit + PINGER_MARGIN);
    match pinger.ping(PingSequence(0), &PAYLOAD).await {
        Ok((_, elapsed)) => Ok((ip, elapsed)),
        Err(SurgeError::IOError(error)) => Err(Failure::Connect(error.kind())),
        Err(_) => Err(Failure::NoReply),
    }
}
