//! The containers of a device, read-only (HOST-001, HOST-010; R-149, stage 1 of hosted
//! applications).
//!
//! The hub runs two fixed `docker` commands through its SSH link (CHK-012): the list of the
//! containers, and the last lines of the log of one. The only operator values that reach a
//! command are a container identifier, rendered by the hub from its hex digits, and a number
//! of lines. Nothing is kept: no table, no event, no log line of the hub holds a container
//! name, an image or a line of a log (LOG-007). No error holds text from the device.
//! No domain imports this one; `schema.rs` merges [`graphql`].

mod docker;
pub mod graphql;
mod model;
#[cfg(test)]
mod tests;

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub use self::model::{
    Container, ContainerId, ContainerLog, ContainerState, Health, LineCount, LogLine, Tail,
};
use crate::{
    fleet::{
        credentials::Vault,
        devices::{self, Device},
        ssh::{self, CommandOutput, FixedCommand, LinkError},
        targets::AddressPolicy,
    },
    ids::DeviceId,
};

/// The longest the list may take on the device.
const LIST_LIMIT: Duration = Duration::from_secs(10);
/// The longest the read of a log may take on the device.
const LOG_LIMIT: Duration = Duration::from_secs(15);

/// What a link needs besides the device.
#[derive(Clone, Copy)]
pub struct Reach<'a> {
    pub db: &'a PgPool,
    pub vault: &'a Vault,
    pub policy: &'a AddressPolicy,
}

/// Why the containers of a device or a log could not be read. No variant holds a host, an
/// address, a container name or the device's output (API-010, LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("device {0} not found")]
    NotFound(DeviceId),
    #[error("a container identifier is 12 to 64 lower-case hexadecimal characters")]
    InvalidContainerId,
    #[error("the number of log lines is 1 to 1000")]
    InvalidLineCount,
    /// The link to the device did not open, or the command did not run on it. A device with
    /// no SSH credential has no list in stage 1 (`LinkError::NoCredential`).
    #[error("the SSH link to the device failed")]
    Link(#[source] LinkError),
    /// The device's shell found no `docker` program.
    #[error("docker is not installed on the device")]
    NoDocker,
    /// `docker` ran and ended without exit status 0.
    #[error("docker on the device did not succeed")]
    DockerFailed,
    #[error("the device has no container with this identifier")]
    NoContainer,
    /// The list is not what `docker ps` writes, or it passed the link's cap.
    #[error("the device's answer cannot be read")]
    Unreadable,
    #[error("device store error")]
    Devices(#[source] devices::Error),
}

/// The containers of `device`, running or not, in Docker's order (newest first).
///
/// # Errors
/// `NotFound` for an unknown device; the [`Error`] for the first step that fails.
pub async fn list(
    reach: Reach<'_>,
    device: DeviceId,
    now: DateTime<Utc>,
) -> Result<Vec<Container>, Error> {
    let device = known(reach.db, device).await?;
    let output = run(reach, &device, &docker::list_command(), (LIST_LIMIT, now)).await?;
    docker::containers(&output)
}

/// The last lines of the log of one container of `device`, both output streams, oldest first.
///
/// # Errors
/// `NotFound` for an unknown device, `NoContainer` when Docker knows no such container; the
/// [`Error`] for the first step that fails.
pub async fn log(
    reach: Reach<'_>,
    device: DeviceId,
    tail: &Tail,
    now: DateTime<Utc>,
) -> Result<ContainerLog, Error> {
    let device = known(reach.db, device).await?;
    let output = run(reach, &device, &docker::log_command(tail), (LOG_LIMIT, now)).await?;
    docker::log(&output)
}

async fn known(db: &PgPool, device: DeviceId) -> Result<Device, Error> {
    devices::get(db, device).await.map_err(|error| match error {
        devices::Error::NotFound(_) => Error::NotFound(device),
        other => Error::Devices(other),
    })
}

/// Opens a link to `device` and runs the one `command` on it.
async fn run(
    reach: Reach<'_>,
    device: &Device,
    command: &FixedCommand,
    (limit, now): (Duration, DateTime<Utc>),
) -> Result<CommandOutput, Error> {
    let mut link = ssh::link(reach.db, reach.vault, reach.policy, device, now)
        .await
        .map_err(Error::Link)?;
    link.run(command, limit).await.map_err(Error::Link)
}
