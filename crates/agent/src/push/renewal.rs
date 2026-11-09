//! When the agent renews its token (MET-013, K8 Section 5): pure, from the clock.

use chrono::{DateTime, TimeDelta, Utc};

/// Renew when fewer than this remain before expiry.
pub const WINDOW: TimeDelta = TimeDelta::days(30);

/// At most one renewal try per this time.
pub const MIN_GAP: TimeDelta = TimeDelta::hours(1);

/// Whether to renew at `now`: the token expires within [`WINDOW`], and the last try (if any)
/// was at least [`MIN_GAP`] ago. An expired token is still tried: the hub decides.
#[must_use]
pub fn due(expires_at: DateTime<Utc>, now: DateTime<Utc>, last_try: Option<DateTime<Utc>>) -> bool {
    let inside = expires_at - now < WINDOW;
    let rested = last_try.is_none_or(|tried| now - tried >= MIN_GAP);
    inside && rested
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: i64, hour: i64) -> DateTime<Utc> {
        DateTime::UNIX_EPOCH + TimeDelta::days(day) + TimeDelta::hours(hour)
    }

    #[test]
    fn met_013_renews_only_in_the_last_30_days() {
        let expires = at(365, 0);
        assert!(!due(expires, at(334, 0), None));
        assert!(!due(expires, at(335, 0), None), "exactly 30 days left");
        assert!(due(expires, at(335, 1), None));
        assert!(due(expires, at(364, 0), None));
        assert!(due(expires, at(366, 0), None), "expired: the hub decides");
    }

    #[test]
    fn k8_5_renews_at_most_once_an_hour() {
        let expires = at(365, 0);
        let now = at(360, 5);
        assert!(!due(expires, now, Some(now - TimeDelta::minutes(59))));
        assert!(due(expires, now, Some(now - TimeDelta::hours(1))));
    }
}
