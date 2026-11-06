//! Network probes, one per check type (CHK-001, CHK-007, CHK-012).
//!
//! [`probe`] runs one attempt and reports the raw outcome. A network failure is a
//! [`ProbeResult`] with `ok: false`, never an `Err` or a panic. Invert, retries and status
//! belong to the evaluation (CHK-008). Every probe resolves its host once, keeps only the
//! addresses the [`AddressPolicy`] allows and connects to one of those addresses, so DNS
//! rebinding cannot reach a refused address. No probe starts a process or a shell.
//!
//! PUSH and GROUP checks are not probed: the device calls the hub, or the children decide.
//! For them [`probe`] returns a failed result saying so; the scheduler must not call it.

mod dns;
mod http;
mod icmp;
mod outcome;
mod resolve;
mod tcp;
mod tls;

use std::{collections::HashMap, net::IpAddr, sync::Arc};

use chrono::{DateTime, Utc};

pub use http::BODY_CAP;
pub use outcome::{Failure, MESSAGE_MAX, ProbeResult, clean_message};

use crate::{
    checks::config::{CheckConfig, HttpSecrets},
    targets::AddressPolicy,
};

/// The device a probe is aimed at. HTTP and DNS checks name their own target in the
/// configuration and ignore the host.
#[derive(Clone, Debug)]
pub struct ProbeTarget {
    pub host: String,
}

/// Replacements for the network, so tests can run without it. Crate-private: production
/// code outside this crate cannot carry host overrides.
#[derive(Clone, Debug, Default)]
pub(crate) struct Overrides {
    /// Lower-case host name to the addresses it resolves to.
    pub hosts: Arc<HashMap<String, Vec<IpAddr>>>,
    /// The port of every configured DNS server (default 53).
    pub dns_port: Option<u16>,
}

/// What a probe may do and when "now" is.
#[derive(Clone, Debug)]
pub struct ProbeEnv {
    pub policy: Arc<AddressPolicy>,
    /// The instant certificate dates are judged against.
    pub now: DateTime<Utc>,
    pub(crate) overrides: Overrides,
}

impl ProbeEnv {
    #[must_use]
    pub fn new(policy: AddressPolicy, now: DateTime<Utc>) -> Self {
        Self {
            policy: Arc::new(policy),
            now,
            overrides: Overrides::default(),
        }
    }
}

/// Runs one attempt of the check and reports what happened, within the check's time limit.
pub async fn probe(
    target: &ProbeTarget,
    config: &CheckConfig,
    secrets: Option<&HttpSecrets>,
    env: &ProbeEnv,
) -> ProbeResult {
    if config.validate().is_err() {
        return ProbeResult::failed(&Failure::BadConfig);
    }
    within(config, run(target, config, secrets, env)).await
}

/// `attempt`, cut off at the check's time limit. This is the only timer that decides the
/// outcome: an ICMP echo still waiting at the limit had no reply, whichever timer fires first,
/// so the same case always gives the same message (CHK-001).
pub(super) async fn within(
    config: &CheckConfig,
    attempt: impl Future<Output = ProbeResult>,
) -> ProbeResult {
    match tokio::time::timeout(config.timeout(), attempt).await {
        Ok(result) => result,
        Err(_) if matches!(config, CheckConfig::Icmp { .. }) => {
            ProbeResult::failed(&Failure::NoReply)
        }
        Err(_) => ProbeResult::failed(&Failure::TimedOut),
    }
}

async fn run(
    target: &ProbeTarget,
    config: &CheckConfig,
    secrets: Option<&HttpSecrets>,
    env: &ProbeEnv,
) -> ProbeResult {
    let host = target.host.as_str();
    match config {
        CheckConfig::Icmp { ip_version, .. } => {
            icmp::probe(host, *ip_version, config.timeout(), env).await
        }
        CheckConfig::Tcp { port, .. } => tcp::probe(host, *port, env).await,
        CheckConfig::Tls {
            port, server_name, ..
        } => tls::probe(host, *port, server_name.as_deref(), env).await,
        CheckConfig::Http(http) => http::probe(http, secrets, env).await,
        CheckConfig::Dns {
            name,
            record_type,
            resolver,
            expected,
        } => {
            let query = dns::DnsQuery {
                name,
                record_type: *record_type,
                resolver: *resolver,
                expected: expected.as_deref(),
            };
            dns::probe(&query, config.timeout(), env).await
        }
        CheckConfig::Push | CheckConfig::Group => ProbeResult::failed(&Failure::NotProbed),
    }
}
