//! Turns a host into addresses the policy allows. The caller connects to one of the returned
//! addresses and never resolves the name again, so DNS rebinding cannot change the target.

use std::net::IpAddr;

use super::{ProbeEnv, outcome::Failure};
use crate::fleet::{
    checks::config::IpVersion,
    targets::{self, TargetError},
};

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

/// The hub's one lookup ([`targets::lookup`]) with the test host overrides. An IP literal in
/// any number of brackets is read as the check editor accepts it (`config::rules::host`).
async fn lookup(host: &str, env: &ProbeEnv) -> Result<Vec<IpAddr>, Failure> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let host = if bare.parse::<IpAddr>().is_ok() {
        bare
    } else {
        host
    };
    targets::lookup(host, &env.overrides.hosts)
        .await
        .map_err(|error| match error {
            TargetError::Unresolved | TargetError::Refused => Failure::Unresolved,
        })
}
