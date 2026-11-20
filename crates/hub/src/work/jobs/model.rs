//! A job of the work route and the machine that can take it (K10, `contracts/work-route.md`).
//! The contract calls a job an "item of work".

use chrono::{DateTime, Utc};
use daemon_core::wire::{
    Limits, StopReason, Switch, WorkArchitecture, WorkKind, WorkOs, WorkRepository, WorkResult,
    WorkState, WorkType,
};

use crate::ids::{DeviceId, JobId, ProjectId};

/// Where a job is. `Queued` is the hub's own state; the others are the states that a machine
/// reports, but `refused`, which puts the job back in the queue (K10 Sections 5 and 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Queued,
    Claimed,
    Preparing,
    Running,
    Finishing,
    Done,
    Failed,
    Stopped,
}

impl State {
    pub(super) const ALL: [Self; 8] = [
        Self::Queued,
        Self::Claimed,
        Self::Preparing,
        Self::Running,
        Self::Finishing,
        Self::Done,
        Self::Failed,
        Self::Stopped,
    ];

    /// The state as the database and the logs spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Preparing => "preparing",
            Self::Running => "running",
            Self::Finishing => "finishing",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }

    pub(super) fn from_stored(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.as_str() == text)
    }

    /// Whether a machine holds a job in this state by a lease.
    #[must_use]
    pub const fn is_held(self) -> bool {
        matches!(
            self,
            Self::Claimed | Self::Preparing | Self::Running | Self::Finishing
        )
    }

    /// Whether the job has ended.
    #[must_use]
    pub const fn is_final(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Stopped)
    }

    /// The state of the job for a state that a machine reports; `None` for `refused`, which is
    /// not a state of the job.
    pub(super) const fn reported(state: WorkState) -> Option<Self> {
        match state {
            WorkState::Claimed => Some(Self::Claimed),
            WorkState::Preparing => Some(Self::Preparing),
            WorkState::Running => Some(Self::Running),
            WorkState::Finishing => Some(Self::Finishing),
            WorkState::Done => Some(Self::Done),
            WorkState::Failed => Some(Self::Failed),
            WorkState::Stopped => Some(Self::Stopped),
            WorkState::Refused => None,
        }
    }
}

/// What a caller gives to put a job in the queue.
#[derive(Clone, Debug, PartialEq)]
pub struct NewJob {
    pub work_type: WorkType,
    pub kind: WorkKind,
    /// What the machine must be; `None` takes each.
    pub os: Option<WorkOs>,
    pub architecture: Option<WorkArchitecture>,
    pub project: Option<ProjectId>,
    pub repository: WorkRepository,
    /// The object of the work type (K10 Section 6).
    pub parameters: serde_json::Value,
    /// The purposes of the secrets that the job needs; never a value.
    pub secrets: Vec<String>,
    pub limits: Limits,
    /// The one machine that may get the job; no different machine ever gets it (ROU-006).
    pub machine: Option<DeviceId>,
}

/// One job, as the hub holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub id: JobId,
    pub asked: NewJob,
    pub state: State,
    /// The number of the attempt that holds or last held the job; 0 before the first claim.
    pub attempt: u32,
    /// Leases that ended with no report (BLD-006).
    pub lost: u8,
    pub holder: Option<DeviceId>,
    pub last_device: Option<DeviceId>,
    /// The machines that refused the job.
    pub refused_by: Vec<DeviceId>,
    pub lease_ends_at: Option<DateTime<Utc>>,
    pub claimed_at: Option<DateTime<Utc>>,
    /// A stop that the hub tells the holder in its next answer.
    pub stop: Option<StopReason>,
    /// Why the job failed or stopped: a fixed code or a fixed sentence of the hub.
    pub reason: Option<String>,
    pub result: Option<WorkResult>,
    /// The count of bytes of the log that the hub accepted.
    pub log_end: u64,
    pub created_at: DateTime<Utc>,
    pub changed_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// What the hub knows of a machine for work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Machine {
    pub device: DeviceId,
    pub switch: Switch,
    pub unclean: bool,
    /// The switch in the hub (USE-004).
    pub off_by_operator: bool,
    /// The end of the hour with no work (K10 Section 11).
    pub off_until: Option<DateTime<Utc>>,
    /// The first claim with the switch on (USE-008).
    pub first_on_at: Option<DateTime<Utc>>,
    /// The operator excluded the machine: it gets only work that names it (ROU-006).
    pub excluded: bool,
}

/// A job that a machine refused or lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Refused,
    Lost,
}

impl Fault {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Lost => "lost",
        }
    }
}
