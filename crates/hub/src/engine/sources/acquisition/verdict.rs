//! What a source's status says about access, before any provider reads the body: refused, rate
//! limited until a time, or usable (K-E1 Section 7). Pure: the time is a parameter.

use chrono::{DateTime, TimeDelta, Utc};
use reqwest::header::HeaderMap;

/// How long to wait when a rate limit names no time.
const DEFAULT_WAIT: TimeDelta = TimeDelta::minutes(1);
/// The longest wait a source's headers can ask for.
const LONGEST_WAIT: TimeDelta = TimeDelta::hours(24);

/// The rate-limit headers of a reply, as numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    /// `x-ratelimit-remaining`.
    pub remaining: Option<u64>,
    /// `x-ratelimit-reset`, in seconds since the epoch.
    pub reset: Option<i64>,
    /// `retry-after`, in seconds.
    pub retry_after: Option<u64>,
}

fn number<T: std::str::FromStr>(headers: &HeaderMap, name: &str) -> Option<T> {
    headers.get(name)?.to_str().ok()?.trim().parse().ok()
}

impl Limits {
    /// The limits in `headers`; a header that is not a number counts as absent.
    #[must_use]
    pub fn of(headers: &HeaderMap) -> Self {
        Self {
            remaining: number(headers, "x-ratelimit-remaining"),
            reset: number(headers, "x-ratelimit-reset"),
            retry_after: number(headers, "retry-after"),
        }
    }
}

/// What the hub does with a reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Hand it to the provider, whatever its status.
    Usable,
    /// 401: the key or the token is refused.
    Refused,
    /// 403 or 429 with a rate limit: wait until then.
    RateLimited(DateTime<Utc>),
}

/// The verdict on a reply with `status` and `limits` at `now`. A 403 or 429 is a rate limit
/// when `x-ratelimit-remaining` is 0 or `retry-after` is set; `retry-after` wins, then
/// `x-ratelimit-reset`, then one minute. The wait is never more than a day.
#[must_use]
pub fn judge(status: u16, limits: Limits, now: DateTime<Utc>) -> Verdict {
    match status {
        401 => Verdict::Refused,
        403 | 429 if limits.remaining == Some(0) || limits.retry_after.is_some() => {
            let after = limits
                .retry_after
                .and_then(|seconds| i64::try_from(seconds).ok())
                .map(|seconds| now + TimeDelta::seconds(seconds.min(LONGEST_WAIT.num_seconds())));
            let reset = limits
                .reset
                .and_then(|seconds| DateTime::from_timestamp(seconds, 0));
            let until = after.or(reset).unwrap_or(now + DEFAULT_WAIT);
            Verdict::RateLimited(until.clamp(now, now + LONGEST_WAIT))
        }
        _ => Verdict::Usable,
    }
}
