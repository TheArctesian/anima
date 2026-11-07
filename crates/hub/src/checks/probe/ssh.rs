//! Listing checks over the SSH link: one command per attempt, run through
//! [`sessions::link`] and read by the pure parsers of `docker` and `systemd` (D-3).
//!
//! The command output is never logged or stored; a link error's text holds no host, address
//! or secret (LOG-007).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    docker,
    outcome::{Failure, ProbeResult},
    systemd, within,
};
use crate::{
    checks::config::CheckConfig,
    credentials::Vault,
    devices::Device,
    sessions::{self, CommandOutput, FixedCommand, LinkError},
    targets::AddressPolicy,
};

/// What a link needs besides the device.
pub(in crate::checks) struct Reach<'a> {
    pub(in crate::checks) db: &'a PgPool,
    pub(in crate::checks) vault: &'a Vault,
    pub(in crate::checks) policy: &'a AddressPolicy,
}

/// The standard output as text, or the fixed reason it cannot be read: cut at the cap, an
/// exit status other than 0, or not UTF-8.
pub(super) fn readable(output: &CommandOutput) -> Result<&str, Failure> {
    if output.cut {
        return Err(Failure::OutputCut);
    }
    if output.exit_status != Some(0) {
        return Err(Failure::CommandFailed);
    }
    std::str::from_utf8(&output.stdout).map_err(|_| Failure::OutputUnreadable)
}

fn link_failure(error: &LinkError) -> Failure {
    match error {
        LinkError::TimeLimit => Failure::TimedOut,
        other => Failure::Link(other.to_string()),
    }
}

async fn run(
    reach: &Reach<'_>,
    device: &Device,
    command: &FixedCommand,
    (limit, now): (std::time::Duration, DateTime<Utc>),
) -> Result<CommandOutput, Failure> {
    let mut link = sessions::link(reach.db, reach.vault, reach.policy, device, now)
        .await
        .map_err(|error| link_failure(&error))?;
    link.run(command, limit)
        .await
        .map_err(|error| link_failure(&error))
}

/// One attempt of a Docker or systemd check on `device`, within the check's time limit.
pub(in crate::checks) async fn observe_listing(
    reach: &Reach<'_>,
    device: &Device,
    config: &CheckConfig,
    now: DateTime<Utc>,
) -> ProbeResult {
    let (command, read, name): (FixedCommand, fn(&CommandOutput, &str) -> ProbeResult, &str) =
        match config {
            CheckConfig::Docker { container, .. } => (docker::command(), docker::read, container),
            CheckConfig::Systemd { unit, .. } => (systemd::command(), systemd::read, unit),
            _ => return ProbeResult::failed(&Failure::NotProbed),
        };
    let attempt = async {
        match run(reach, device, &command, (config.timeout(), now)).await {
            Ok(output) => read(&output, name),
            Err(failure) => ProbeResult::failed(&failure),
        }
    };
    within(config, attempt).await
}
