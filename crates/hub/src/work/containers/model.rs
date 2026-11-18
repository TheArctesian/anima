//! Domain types of the containers of a device (HOST-001, HOST-010): what `docker ps` says of a
//! container, the last lines of its log, and the two values the operator gives, checked.

use std::fmt;

use chrono::{DateTime, Utc};

use super::Error;
use crate::fleet::ssh::HexDigits;

/// The fewest digits of a container identifier Docker shows: its short form.
const SHORTEST_ID: usize = 12;
/// The most digits: the full identifier.
const LONGEST_ID: usize = 64;
/// The most log lines one read asks for.
const MOST_LINES: u16 = 1000;
/// The log lines a read asks for when the operator names no number.
pub(super) const DEFAULT_LINES: u16 = 200;

/// A container identifier the hub accepts: 12 to 64 lower-case hex digits (HOST-001). It is
/// the only text from the operator that reaches a command, and it reaches it as digits the
/// hub renders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ContainerId(HexDigits);

impl ContainerId {
    /// The identifier in `text`.
    ///
    /// # Errors
    /// [`Error::InvalidContainerId`] when `text` is not 12 to 64 lower-case hex digits.
    pub(super) fn parse(text: &str) -> Result<Self, Error> {
        if !(SHORTEST_ID..=LONGEST_ID).contains(&text.len()) {
            return Err(Error::InvalidContainerId);
        }
        HexDigits::parse(text)
            .map(Self)
            .ok_or(Error::InvalidContainerId)
    }

    pub(super) const fn digits(&self) -> &HexDigits {
        &self.0
    }
}

/// How many of the last log lines to read: 1 to 1000 (HOST-001).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LineCount(u16);

impl LineCount {
    /// `lines`, or [`DEFAULT_LINES`] when it is `None`.
    ///
    /// # Errors
    /// [`Error::InvalidLineCount`] when `lines` is not 1 to 1000.
    pub(super) fn new(lines: Option<i32>) -> Result<Self, Error> {
        let Some(lines) = lines else {
            return Ok(Self(DEFAULT_LINES));
        };
        u16::try_from(lines)
            .ok()
            .filter(|lines| (1..=MOST_LINES).contains(lines))
            .map(Self)
            .ok_or(Error::InvalidLineCount)
    }

    pub(super) fn get(self) -> u32 {
        u32::from(self.0)
    }
}

/// The log read the operator asked for: one container, its last lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Tail {
    pub(super) container: ContainerId,
    pub(super) lines: LineCount,
}

/// What Docker says the container is doing (`State` of `docker ps`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ContainerState {
    Running,
    Exited,
    Restarting,
    Paused,
    Created,
    Removing,
    Dead,
    /// A state this hub does not know, or none.
    Unknown,
}

/// What the status text says of the container's health check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Health {
    Healthy,
    Unhealthy,
    Starting,
    /// No health check, or the container is not running.
    None,
}

/// One container, as one line of `docker ps` describes it. Every text comes from the device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Container {
    pub(super) id: String,
    /// Docker's names; the first is the one it shows.
    pub(super) names: Vec<String>,
    pub(super) image: String,
    pub(super) state: ContainerState,
    pub(super) status: String,
    pub(super) health: Health,
    pub(super) ports: String,
    pub(super) created_at: Option<DateTime<Utc>>,
    pub(super) compose_project: Option<String>,
    pub(super) compose_service: Option<String>,
}

/// The last lines of one container's log. Never kept by the hub and never logged: it can hold
/// a secret (LOG-007).
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ContainerLog {
    /// Oldest first.
    pub(super) lines: Vec<String>,
    /// The output reached the link's cap: the lines are the oldest of the tail and the newest
    /// are missing.
    pub(super) cut: bool,
}

/// Shows the number of lines, never their text, so a logged value holds no log (LOG-007).
impl fmt::Debug for ContainerLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContainerLog")
            .field("lines", &self.lines.len())
            .field("cut", &self.cut)
            .finish()
    }
}
