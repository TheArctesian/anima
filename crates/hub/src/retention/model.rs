//! The kinds of data that have a retention period, and the error a bad period gives.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

/// What a retention period applies to, in the order the operator sees them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "RetentionSubject")]
pub enum Subject {
    Heartbeats,
    Metrics,
    Events,
    Recordings,
    Deliveries,
}

impl Subject {
    pub const ALL: [Self; 5] = [
        Self::Heartbeats,
        Self::Metrics,
        Self::Events,
        Self::Recordings,
        Self::Deliveries,
    ];

    /// The text stored in `retention.periods.subject`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Heartbeats => "heartbeats",
            Self::Metrics => "metrics",
            Self::Events => "events",
            Self::Recordings => "recordings",
            Self::Deliveries => "deliveries",
        }
    }

    pub(super) fn from_stored(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|subject| subject.as_str() == text)
    }
}

/// An input field the operator must fix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    pub field: &'static str,
    pub message: String,
}
