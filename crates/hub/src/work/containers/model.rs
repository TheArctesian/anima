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

/// The one state of a Compose project, from the states of its containers (HOST-019; K1
/// `ComposeProjectState`). `project.rs` has the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ComposeProjectState")]
pub enum ProjectState {
    /// Every container runs.
    Running,
    /// Every container runs and one fails its health check.
    Unhealthy,
    /// Some containers run and some do not.
    PartlyRunning,
    /// A container restarts or is dead.
    Failing,
    /// No container runs and one is paused.
    Paused,
    /// No container runs.
    Stopped,
    /// A container has a state this hub does not know.
    Unknown,
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
    /// The folder from which Compose started the project, as Docker says; `None` with no
    /// project, or when the project was started with no such label.
    pub compose_working_dir: Option<String>,
    /// The Compose files of the project, as paths on the device, in the order Compose read
    /// them. Anima reads this and changes no file (R-211).
    pub compose_files: Vec<String>,
    /// The one state of the container's Compose project (HOST-019); `None` with no project.
    pub project_state: Option<ProjectState>,
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

/// The details of one container (HOST-017), as `docker container inspect` and `docker top`
/// describe it. Every text comes from the device. Never kept by the hub and never logged: a
/// command or a label can hold a secret (LOG-007).
#[derive(Clone, PartialEq, Eq)]
pub struct ContainerDetail {
    pub id: String,
    pub name: String,
    /// The image as the container names it.
    pub image: String,
    /// The identifier of the image (`sha256:` and 64 digits).
    pub image_id: String,
    /// The program and its arguments.
    pub command: Vec<String>,
    pub working_dir: String,
    /// The user the program runs as; empty for the default of the image.
    pub user: String,
    /// `no`, `always`, `unless-stopped`, or `on-failure` with its limit (`on-failure:3`).
    pub restart_policy: String,
    pub state: ContainerState,
    /// The exit status of the last run; 0 while the container runs.
    pub exit_code: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    /// How many times Docker restarted the container.
    pub restart_count: u32,
    pub health_check: Option<HealthCheck>,
    pub health: Health,
    /// The health checks that failed one after the other, to now.
    pub failing_streak: u32,
    pub last_health_run: Option<HealthRun>,
    pub mounts: Vec<Mount>,
    pub networks: Vec<NetworkAddress>,
    pub labels: Vec<Label>,
    /// The names of the environment variables. The hub never reads a value (HOST-017).
    pub environment_names: Vec<String>,
    /// The processes of a running container; empty when it does not run.
    pub processes: Vec<Process>,
}

/// Shows the identifier and no text of the device, so a logged value holds none (LOG-007).
impl fmt::Debug for ContainerDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContainerDetail")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// The health check Docker runs in a container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthCheck {
    /// The test as Docker keeps it: `CMD` or `CMD-SHELL`, then the command.
    pub test: Vec<String>,
    pub interval_seconds: Option<u32>,
}

/// The last run of a health check: its end and its exit status. The hub does not read what
/// the check wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealthRun {
    pub ended_at: Option<DateTime<Utc>>,
    pub exit_code: i32,
}

/// One mount of a container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mount {
    /// `volume`, `bind` or `tmpfs`, as Docker writes it.
    pub kind: String,
    /// The name of the volume; `None` for a mount that is not a volume.
    pub name: Option<String>,
    /// The path on the device.
    pub source: String,
    /// The path in the container.
    pub destination: String,
    pub read_only: bool,
}

/// The addresses of a container on one network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkAddress {
    pub network: String,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub gateway: Option<String>,
    pub mac_address: Option<String>,
}

/// One label of a container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub value: String,
}

/// One process of a running container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub user: Option<String>,
    pub command: String,
}

/// Whether a newer image exists for a container (K1 `ContainerImageUpdateState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ContainerImageUpdateState")]
pub enum UpdateState {
    /// The registry gives the digest that the device has for the tag.
    UpToDate,
    /// The registry gives a different digest for the tag: a pull would get a newer image.
    NewerImage,
    /// The hub cannot say; [`NotKnown`] gives the reason.
    NotKnown,
}

/// Why the hub cannot say whether a newer image exists (K1 `ContainerImageUpdateReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ContainerImageUpdateReason")]
pub enum NotKnown {
    /// The container names its image by a digest: it asked for that image and no newer one.
    PinnedDigest,
    /// The image is of a registry that Anima does not read.
    OtherRegistry,
    /// The image has no name of a registry: an identifier, or a name no registry accepts.
    NoName,
    /// Docker keeps no registry digest for the tag: the image was built on the device, or
    /// the tag is not on the device now.
    NoLocalDigest,
    /// The registry has no such tag for a reader with no account: private, or gone.
    NotAtRegistry,
    /// The registry was not read: it did not answer in time, the hub may not reach it, or
    /// the device has more tags than one read asks for.
    RegistryNotRead,
}
