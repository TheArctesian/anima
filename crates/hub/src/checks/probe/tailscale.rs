//! Tailscale ping: a disco ping the hub asks the `LocalAPI` proxy to send (D-4, CHK-009).
//!
//! The proxy is operator infrastructure: it sits on the deny list on purpose, so its address
//! is never judged by the [`AddressPolicy`](crate::targets::AddressPolicy). The ping target is
//! judged: only an address of 100.64.0.0/10 that the policy permits is sent, because the proxy
//! accepts nothing else. The proxy's reply is never logged or stored; the result holds only
//! fixed words and the latency (LOG-007, WP-23 H-5).

use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use reqwest::{Client, Response, StatusCode, redirect::Policy};
use serde::Deserialize;

use super::{
    ProbeEnv,
    outcome::{Failure, ProbeResult},
    resolve::allowed_addresses,
};
use crate::checks::config::IpVersion;

/// The most reply bytes read (64 KiB); a `PingResult` is far smaller.
pub(super) const REPLY_CAP: usize = 64 * 1024;

/// The part of the proxy's `PingResult` the check reads.
#[derive(Deserialize)]
struct PingReply {
    /// Empty when the peer answered; the text is never kept.
    #[serde(rename = "Err", default)]
    error: Option<String>,
    #[serde(rename = "LatencySeconds", default)]
    latency_seconds: Option<f64>,
}

/// Whether the address is in 100.64.0.0/10, the range Tailscale assigns.
fn in_tailnet(ip: Ipv4Addr) -> bool {
    let [first, second, ..] = ip.octets();
    first == 100 && second & 0b1100_0000 == 0b0100_0000
}

/// The one address to ping: resolved once, IPv4, inside the tailnet range, permitted.
async fn tailnet_address(host: &str, env: &ProbeEnv) -> Result<Ipv4Addr, Failure> {
    let found = match allowed_addresses(host, Some(IpVersion::V4), env).await {
        Ok(found) => found,
        Err(Failure::NoAddress | Failure::NotPermitted) => {
            return Err(Failure::NoTailnetAddress);
        }
        Err(other) => return Err(other),
    };
    found
        .into_iter()
        .find_map(|ip| match ip {
            IpAddr::V4(v4) if in_tailnet(v4) => Some(v4),
            _ => None,
        })
        .ok_or(Failure::NoTailnetAddress)
}

/// Reads at most [`REPLY_CAP`] bytes; one more byte than that is a failure.
async fn read_capped(response: &mut Response) -> Result<Vec<u8>, Failure> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Request)? {
        body.extend_from_slice(&chunk);
        if body.len() > REPLY_CAP {
            return Err(Failure::ProxyReplyTooLong);
        }
    }
    Ok(body)
}

/// The latency of an answered ping, or the fixed reason the reply is not one.
fn read_reply(body: &[u8]) -> Result<Duration, Failure> {
    let reply: PingReply =
        serde_json::from_slice(body).map_err(|_| Failure::ProxyReplyUnreadable)?;
    if reply.error.is_some_and(|text| !text.is_empty()) {
        return Err(Failure::PingFailed);
    }
    reply
        .latency_seconds
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .ok_or(Failure::ProxyReplyUnreadable)
}

fn client() -> Result<Client, Failure> {
    Client::builder()
        .user_agent("anima-hub")
        .no_proxy()
        .redirect(Policy::none())
        .build()
        .map_err(|_| Failure::BadConfig)
}

async fn ask(localapi: &str, ip: Ipv4Addr) -> Result<Duration, Failure> {
    let url = format!("http://{localapi}/localapi/v0/ping?ip={ip}&type=disco");
    let mut response = client()?
        .post(url)
        .send()
        .await
        .map_err(|_| Failure::Request)?;
    match response.status() {
        StatusCode::OK => read_reply(&read_capped(&mut response).await?),
        StatusCode::FORBIDDEN => Err(Failure::ProxyRefused),
        other => Err(Failure::Status(other.as_u16())),
    }
}

/// One attempt. `target` is the configured target, else `host` of the device. The caller
/// cuts the attempt off at the check's time limit.
pub(super) async fn probe(
    target: Option<&str>,
    host: &str,
    localapi: Option<&str>,
    env: &ProbeEnv,
) -> ProbeResult {
    let Some(localapi) = localapi else {
        return ProbeResult::unjudged(&Failure::NeedsLocalApi);
    };
    let ip = match tailnet_address(target.unwrap_or(host), env).await {
        Ok(ip) => ip,
        Err(failure) => return ProbeResult::failed(&failure),
    };
    match ask(localapi, ip).await {
        Ok(latency) => ProbeResult::up(latency).via(IpAddr::V4(ip)),
        Err(failure) => ProbeResult::failed(&failure),
    }
}
