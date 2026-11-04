//! Turns a host into addresses the policy allows. The caller connects to one of the returned
//! addresses and never resolves the name again, so DNS rebinding cannot change the target.

use std::net::IpAddr;

use super::{ProbeEnv, outcome::Failure};
use crate::checks::config::IpVersion;

/// Resolves `host` once and keeps only the addresses of `family` (if given) that the policy
/// allows.
///
/// # Errors
/// [`Failure::Unresolved`] when the name has no address, [`Failure::NoAddress`] when none
/// is of `family`, [`Failure::NotPermitted`] when the policy refuses all of them.
pub(super) async fn allowed_addresses(
    host: &str,
    family: Option<IpVersion>,
    env: &ProbeEnv,
) -> Result<Vec<IpAddr>, Failure> {
    let mut found = lookup(host, env).await?;
    if let Some(version) = family {
        found.retain(|ip| {
            matches!(
                (version, ip),
                (IpVersion::V4, IpAddr::V4(_)) | (IpVersion::V6, IpAddr::V6(_))
            )
        });
        if found.is_empty() {
            return Err(Failure::NoAddress);
        }
    }
    found.retain(|ip| env.policy.permits(*ip));
    if found.is_empty() {
        Err(Failure::NotPermitted)
    } else {
        Ok(found)
    }
}

async fn lookup(host: &str, env: &ProbeEnv) -> Result<Vec<IpAddr>, Failure> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return Ok(vec![ip]);
    }
    let found: Vec<IpAddr> = match env.overrides.hosts.get(&host.to_ascii_lowercase()) {
        Some(ips) => ips.clone(),
        None => tokio::net::lookup_host((host, 0))
            .await
            .map_err(|_| Failure::Unresolved)?
            .map(|addr| addr.ip())
            .collect(),
    };
    if found.is_empty() {
        Err(Failure::Unresolved)
    } else {
        Ok(found)
    }
}
