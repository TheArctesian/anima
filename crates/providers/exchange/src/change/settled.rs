//! What came of a change that was sent, as a closed set of outcomes (design C.1 and C.6).

use chrono::{DateTime, Utc};

use crate::exchange::Body;
use crate::parts::CommitSha;

/// The reply to a sent change, as far as the provider reads it. The engine reads the headers
/// into [`Limit`], so the provider sees no header text.
#[derive(Clone, Copy, Debug)]
pub struct ChangeReply<'a> {
    pub status: u16,
    pub body: Body<'a>,
    pub limit: Limit,
    /// When the reply came: the time `retry_after_seconds` counts from.
    pub at: DateTime<Utc>,
}

/// The rate limit headers of a reply. `None` when the reply had no such header or it was not
/// a number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limit {
    /// `x-ratelimit-remaining`.
    pub remaining: Option<u64>,
    /// `x-ratelimit-reset`.
    pub reset_at: Option<DateTime<Utc>>,
    /// `retry-after`, in seconds.
    pub retry_after_seconds: Option<u64>,
}

/// What came of a change. Fixed outcomes: none holds text from the reply (LOG-007).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Settled {
    /// The provider made the change.
    Done(Receipt),
    /// The provider refused it: nothing was changed.
    Refused(Refusal),
    /// The provider limits requests until `until`: nothing was changed.
    Limited { until: DateTime<Utc> },
    /// The reply does not say whether the change was made.
    Unknown,
}

/// What the provider gave back for a change it made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Receipt {
    /// Made, with nothing to read back.
    Made,
    /// Made a new thing: an issue's number, or a comment's ID.
    Created { number: u64 },
    /// Merged as this commit.
    Merged { sha: CommitSha },
}

/// Why the provider refused a change. Each kind has fixed words in the hub (design C.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The provider refused the key: connect the account again.
    KeyRefused,
    /// The App is not permitted to do it.
    NotPermitted,
    /// No such thing, or the App may not read it.
    NotFound,
    /// The pull request's head moved since it was read.
    HeadMoved,
    /// The pull request cannot be merged now, or not with that method.
    NotMergeable,
    /// The provider refused the branch, the inputs or another part.
    Invalid,
    /// The feature is off in the repository, for example issues.
    Gone,
    /// The thing is not in a state that allows the change.
    Conflict,
    /// A refusal with no rule of its own.
    Unexplained,
}
