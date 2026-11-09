//! The status rule of a domain expiry check (D-29). Pure: `now` comes in as a parameter.
//!
//! Up while the days left are at least the warning days; Degraded below them; Down once the
//! registration has expired; Unknown when RDAP gives no expiry date. The evaluation applies
//! invert to the result as for every check, but never retries: a domain expiry check has no
//! retries, so a failure settles at once (R-102).

use chrono::{DateTime, Utc};

use crate::checks::{
    model::whole_days,
    probe::outcome::{Failure, Judgement, ProbeResult, clean_message},
};

/// The probe result for the expiry date `not_after` at `now`. The result carries the date, so
/// the check shows its days left; it has no response time, because the RDAP server's speed
/// says nothing about the domain.
pub(super) fn judge(
    not_after: Option<DateTime<Utc>>,
    warn_days: u32,
    now: DateTime<Utc>,
) -> ProbeResult {
    let Some(not_after) = not_after else {
        return ProbeResult::unjudged(&Failure::NoExpiryDate);
    };
    let days = whole_days(not_after, now);
    let mut result = if not_after <= now {
        ProbeResult::failed(&Failure::DomainExpired)
    } else if i64::from(days) < i64::from(warn_days) {
        ProbeResult {
            message: Some(clean_message(&Failure::DomainExpiring.to_string())),
            judgement: Judgement::Expiring,
            ..ProbeResult::up_unmeasured()
        }
    } else {
        ProbeResult::up_unmeasured()
    };
    result.domain_not_after = Some(not_after);
    result
}
