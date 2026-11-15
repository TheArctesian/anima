//! What a live read or change takes and gives (K-E1 Section 13.1).

use std::{fmt, sync::Arc};

use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::{Body, Response};

/// Whether a body says it will not change soon. Pure.
pub type Settles = fn(body: &[u8]) -> bool;

/// Whether a body may be kept at all. Pure.
pub type Holdable = fn(body: &[u8]) -> bool;

/// How long an answer stays fresh in the cache, and whether this read must be new.
#[derive(Clone, Copy, Debug)]
pub struct Keep {
    /// `None`: never kept (file content).
    fresh_for: Option<TimeDelta>,
    /// A longer window for an answer `settled` says will not change soon.
    settled: Option<(TimeDelta, Settles)>,
    /// Only an answer this says may be kept is kept; any other is never held.
    holds: Option<Holdable>,
    forced: bool,
}

impl Keep {
    /// Kept and served for `seconds` after it was read.
    #[must_use]
    pub const fn for_seconds(seconds: i64) -> Self {
        Self {
            fresh_for: Some(TimeDelta::seconds(seconds)),
            settled: None,
            holds: None,
            forced: false,
        }
    }

    /// Never kept, never served from the cache, never served stale: file content.
    #[must_use]
    pub const fn never() -> Self {
        Self {
            fresh_for: None,
            settled: None,
            holds: None,
            forced: false,
        }
    }

    /// Kept only when `holds` says of the body that it may be: a directory read answered with
    /// a file's content is never held (K-E1 Section 13.1). `holds` must be pure.
    #[must_use]
    pub const fn only_when(self, holds: Holdable) -> Self {
        Self {
            holds: Some(holds),
            ..self
        }
    }

    /// Kept for `seconds` instead when `settled` says of the body that it will not change soon
    /// (a completed run, checks with none pending). `settled` must be pure.
    #[must_use]
    pub const fn longer_when(self, seconds: i64, settled: Settles) -> Self {
        Self {
            settled: Some((TimeDelta::seconds(seconds), settled)),
            ..self
        }
    }

    /// `true`: read the source again unless the held answer is less than five seconds old
    /// (the client's Refresh).
    #[must_use]
    pub const fn fresh(self, fresh: bool) -> Self {
        Self {
            forced: fresh,
            ..self
        }
    }

    pub(super) const fn is_kept(self) -> bool {
        self.fresh_for.is_some()
    }

    pub(super) const fn is_forced(self) -> bool {
        self.forced
    }

    /// How long `body` stays fresh; `None` when it is not kept.
    pub(super) fn fresh_for(self, body: &[u8]) -> Option<TimeDelta> {
        let short = self.fresh_for?;
        if self.holds.is_some_and(|holds| !holds(body)) {
            return None;
        }
        Some(match self.settled {
            Some((long, settled)) if settled(body) => long,
            _ => short,
        })
    }
}

/// Whether a change may spend the requests the hub keeps for the sync (K-E1 Section 13.1).
/// The caller decides from who asked; the engine knows no token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reserve {
    /// The operator's own change (a login or browser token): it may go down to 0.
    Spendable,
    /// A script's or an agent's change (an API key): it stops at the reserve, as a read does.
    Kept,
}

/// Why a live read or change got no answer. Fixed words only: never a token, a path, a body
/// or a reply of the source (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LiveFailure {
    #[error("no source account has this ID")]
    NotFound,
    #[error("the account is paused")]
    Paused,
    /// A 401, or no usable stored key: connect the account again.
    #[error("the source refused the key")]
    KeyRefused,
    /// The source limits requests, or the reads reached the sync's reserve, until `until`.
    #[error("the source limits requests")]
    Limited { until: DateTime<Utc> },
    /// Nothing was sent: the source could not be reached.
    #[error("the source could not be reached")]
    Unreachable,
    /// A 5xx, or no token came back.
    #[error("the source's answer could not be used")]
    Unusable,
    /// Nothing was sent: the path, the host or the size of the body is not one the hub sends.
    #[error("the hub does not send this request")]
    Refused,
    /// The source's declaration names no changes.
    #[error("the source makes no changes")]
    NoChanges,
    /// The account's earlier requests took too long to leave room for this one.
    #[error("the hub is still busy with this account's requests")]
    Busy,
    #[error("the hub could not read the account")]
    Store,
}

impl LiveFailure {
    /// Whether a held answer may stand in for the failed read.
    pub(super) const fn passes(self) -> bool {
        matches!(
            self,
            Self::Limited { .. } | Self::Unreachable | Self::Unusable | Self::Busy | Self::Store
        )
    }
}

/// An answer of the source, from the cache or new.
#[derive(Clone)]
pub struct Reply {
    pub(super) status: u16,
    /// `None`: larger than the hub reads.
    pub(super) body: Option<Arc<[u8]>>,
    pub(super) at: DateTime<Utc>,
    /// Set when the newest read failed and this is the last good answer.
    pub(super) problem: Option<LiveFailure>,
}

impl Reply {
    /// The status and body, as a provider's parser reads them.
    #[must_use]
    pub fn response(&self) -> Response<'_> {
        Response {
            status: self.status,
            body: self.body.as_deref().map_or(Body::TooLarge, Body::Read),
        }
    }

    /// When the source gave this answer.
    #[must_use]
    pub const fn at(&self) -> DateTime<Utc> {
        self.at
    }

    /// The newest read failed and this is the last good answer, at most an hour old.
    #[must_use]
    pub const fn stale(&self) -> bool {
        self.problem.is_some()
    }

    /// Why the newest read failed, when [`Self::stale`].
    #[must_use]
    pub const fn problem(&self) -> Option<LiveFailure> {
        self.problem
    }
}

/// The body is never shown (LOG-007): only its length.
impl fmt::Debug for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reply")
            .field("status", &self.status)
            .field("bytes", &self.body.as_ref().map(|body| body.len()))
            .field("at", &self.at)
            .field("problem", &self.problem)
            .finish()
    }
}

/// Whether an account can be read now, from its stored row (design B.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Ready,
    /// No key is stored, or the last sync's key was refused.
    NeedsReconnect,
    Paused,
    /// No such account.
    NotFound,
}
