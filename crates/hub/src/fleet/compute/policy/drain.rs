//! The drain of a node that a policy stops (CAP-013): it gets no new requests, and its current
//! requests continue for [`DRAIN_LIMIT`] or less. Pure: a state with a deadline; the caller
//! gives the time. The daemon side is task L1-6.

use chrono::{DateTime, TimeDelta, Utc};

/// The longest time that current requests continue after a policy stops a node.
pub const DRAIN_LIMIT: TimeDelta = TimeDelta::seconds(60);

/// The state of a node for model requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drain {
    /// The node takes new requests.
    Serving,
    /// The node takes no new requests; current requests continue until `until`.
    Draining { until: DateTime<Utc> },
    /// The node takes no requests and runs none.
    Stopped,
}

impl Drain {
    /// The next state at `now`, when the policy permits model work or not.
    #[must_use]
    pub fn next(self, permitted: bool, now: DateTime<Utc>) -> Self {
        match (self, permitted) {
            (_, true) => Self::Serving,
            (Self::Serving, false) => Self::Draining {
                until: now + DRAIN_LIMIT,
            },
            (Self::Draining { until }, false) if now < until => self,
            (Self::Draining { .. } | Self::Stopped, false) => Self::Stopped,
        }
    }

    /// Whether the node gets a new request.
    #[must_use]
    pub const fn takes_new(self) -> bool {
        matches!(self, Self::Serving)
    }

    /// Whether a current request may continue at `now`.
    #[must_use]
    pub fn runs_current(self, now: DateTime<Utc>) -> bool {
        match self {
            Self::Serving => true,
            Self::Draining { until } => now < until,
            Self::Stopped => false,
        }
    }
}
