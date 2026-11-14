//! What each account may still spend of its rate limit on live reads (K-E1 Section 13.1;
//! decisions 5 and 7 of the slice). Pure: the time is a parameter (CQ-021).
//!
//! The hub keeps the last `x-ratelimit-remaining` and `x-ratelimit-reset` seen on a live answer
//! and assumes no total: the reserve is a count of requests left, so it holds whether the
//! installation may send 5,000 or 12,500 an hour. A remaining count may rise from one answer
//! to the next, so the last answer always wins.

use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};

use crate::{
    engine::sources::acquisition::verdict::{Limits, Verdict, judge},
    ids::SourceAccountId,
};

/// Live reads stop when this many requests or fewer are left: they belong to the sync.
pub(super) const RESERVE: u64 = 500;
/// How long a stop lasts when the answer named no reset time.
const NO_RESET_WAIT: TimeDelta = TimeDelta::minutes(1);

/// Until when a reply with `status` and `limits` at `now` says the source limits requests, or
/// `None` when it does not (decision 7): a 429 always; a 403 when no request is left or it
/// names a `retry-after`. Any other 403 is a refusal for the provider to read.
pub(super) fn limited_until(
    status: u16,
    limits: Limits,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    match judge(status, limits, now) {
        Verdict::RateLimited(until) => Some(until),
        Verdict::Usable | Verdict::Refused if status == 429 => Some(
            limits
                .reset
                .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
                .filter(|reset| *reset > now)
                .unwrap_or(now + NO_RESET_WAIT),
        ),
        Verdict::Usable | Verdict::Refused => None,
    }
}

#[derive(Clone, Copy, Debug)]
struct Seen {
    remaining: u64,
    /// When the count starts again.
    reset_at: DateTime<Utc>,
}

/// The last count seen for each account.
#[derive(Debug, Default)]
pub(super) struct Budget {
    seen: HashMap<SourceAccountId, Seen>,
}

impl Budget {
    /// Notes the rate-limit headers of an answer read at `now`. An answer without a remaining
    /// count changes nothing.
    pub(super) fn note(&mut self, account: SourceAccountId, limits: Limits, now: DateTime<Utc>) {
        let Some(remaining) = limits.remaining else {
            return;
        };
        let reset_at = limits
            .reset
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
            .unwrap_or(now + NO_RESET_WAIT);
        self.seen.insert(
            account,
            Seen {
                remaining,
                reset_at,
            },
        );
    }

    /// Notes that the source limits the account's requests until `until`.
    pub(super) fn limited(&mut self, account: SourceAccountId, until: DateTime<Utc>) {
        self.seen.insert(
            account,
            Seen {
                remaining: 0,
                reset_at: until,
            },
        );
    }

    /// Whether a live read of `account` may go at `now`; `Err` holds the time the count starts
    /// again. Changes do not ask: they may spend the reserve.
    pub(super) fn read(
        &self,
        account: SourceAccountId,
        now: DateTime<Utc>,
    ) -> Result<(), DateTime<Utc>> {
        match self.seen.get(&account) {
            Some(seen) if seen.remaining <= RESERVE && now < seen.reset_at => Err(seen.reset_at),
            _ => Ok(()),
        }
    }
}
