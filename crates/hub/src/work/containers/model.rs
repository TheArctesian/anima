//! Domain types of the containers of a device (HOST-001, HOST-010): what `docker ps` says of a
//! container, the last lines of its log, and the two values the operator gives, checked.

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::fmt;

use async_graphql::Enum;
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
const DEFAULT_LINES: u16 = 200;

/// A container identifier the hub accepts: 12 to 64 lower-case hex digits (HOST-001). It is
/// the only text from the operator that reaches a command, and it reaches it as digits the
/// hub renders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerId(HexDigits);

impl ContainerId {
    /// The identifier in `text`.
    ///
    /// # Errors
    /// [`Error::InvalidContainerId`] when `text` is not 12 to 64 lower-case hex digits.
    pub fn parse(text: &str) -> Result<Self, Error> {
        if !(SHORTEST_ID..=LONGEST_ID).contains(&text.len()) {
            return Err(Error::InvalidContainerId);
        }
        HexDigits::parse(text)
            .map(Self)
            .ok_or(Error::InvalidContainerId)
    }

    #[must_use]
    pub const fn digits(&self) -> &HexDigits {
        &self.0
    }
}

/// How many of the last log lines to read: 1 to 1000 (HOST-001).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCount(u16);

impl LineCount {
    /// `lines`, or [`DEFAULT_LINES`] when it is `None`.
    ///
    /// # Errors
    /// [`Error::InvalidLineCount`] when `lines` is not 1 to 1000.
    pub fn new(lines: Option<i32>) -> Result<Self, Error> {
        let Some(lines) = lines else {
            return Ok(Self(DEFAULT_LINES));
        };
        u16::try_from(lines)
            .ok()
            .filter(|lines| (1..=MOST_LINES).contains(lines))
            .map(Self)
            .ok_or(Error::InvalidLineCount)
    }

    #[must_use]
    pub fn get(self) -> u32 {
        u32::from(self.0)
    }
}

/// The log read the operator asked for: one container, its last lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tail {
    pub container: ContainerId,
    pub lines: LineCount,
}

/// What Docker says the container is doing (`State` of `docker ps`; K1 `ContainerState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum ContainerState {
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

/// What the status text says of the container's health check (K1 `ContainerHealth`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ContainerHealth")]
pub enum Health {
    Healthy,
    Unhealthy,
    Starting,
    /// No health check, or the container is not running.
    None,
}

/// One container, as one line of `docker ps` describes it. Every text comes from the device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub id: String,
    /// The first name Docker shows.
    pub name: String,
    pub image: String,
    pub state: ContainerState,
    pub status: String,
    pub health: Health,
    /// Each published or exposed port, as Docker writes it (`0.0.0.0:8080->80/tcp`).
    pub ports: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
}

/// One line of a container's log: what the container wrote to either output stream.
#[derive(Clone, PartialEq, Eq)]
pub struct LogLine {
    /// When Docker took the line; `None` when the line has no time Docker wrote.
    pub at: Option<DateTime<Utc>>,
    pub text: String,
}

/// The last lines of one container's log. Never kept by the hub and never logged: it can hold
/// a secret (LOG-007).
#[derive(Clone, PartialEq, Eq)]
pub struct ContainerLog {
    /// Oldest first.
    pub lines: Vec<LogLine>,
    /// The output reached the link's cap: the lines are the oldest of the tail and the newest
    /// are missing.
    pub cut: bool,
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
