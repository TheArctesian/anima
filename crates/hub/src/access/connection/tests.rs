use std::time::Duration;

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};
use uuid::Uuid;

use super::{Closing, Lease, Verdict, lease_of, next_check};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

const RECHECK: Duration = Duration::from_secs(15);

#[test]
fn r_020_a_lease_is_checked_every_interval_and_at_its_expiry() {
    let far = now() + TimeDelta::days(30);
    assert_eq!(next_check(far, now(), RECHECK), Some(RECHECK));
    let soon = now() + TimeDelta::milliseconds(1500);
    assert_eq!(
        next_check(soon, now(), RECHECK),
        Some(Duration::from_millis(1500))
    );
    let exactly = now() + TimeDelta::seconds(15);
    assert_eq!(next_check(exactly, now(), RECHECK), Some(RECHECK));
}

#[test]
fn r_020_a_lease_ends_at_its_expiry() {
    assert_eq!(next_check(now(), now(), RECHECK), None);
    assert_eq!(
        next_check(now() - TimeDelta::seconds(1), now(), RECHECK),
        None
    );
    let just_before = now() + TimeDelta::microseconds(1);
    assert_eq!(
        next_check(just_before, now(), RECHECK),
        Some(Duration::from_micros(1))
    );
}

#[test]
fn r_020_only_a_granted_lease_opens_the_connection() {
    let lease = Lease {
        token: Uuid::now_v7(),
        expires_at: now(),
    };
    assert_eq!(lease_of(Some(Verdict::Leased(lease))), Ok(lease));
    assert_eq!(lease_of(Some(Verdict::Refused)), Err(Closing::Forbidden));
    assert_eq!(lease_of(None), Err(Closing::Forbidden));
    assert_eq!(
        lease_of(Some(Verdict::Unavailable)),
        Err(Closing::Unavailable)
    );
}

#[test]
fn r_020_close_frames_have_fixed_reasons_and_graphql_ws_codes() {
    let frames = [
        (Closing::ClientLeft, None),
        (Closing::Unreadable, Some((1008, "Message refused"))),
        (
            Closing::InitTimeout,
            Some((4408, "Connection initialisation timeout")),
        ),
        (Closing::Forbidden, Some((4403, "Forbidden"))),
        (
            Closing::Unavailable,
            Some((1011, "Sign-in cannot be checked")),
        ),
        (Closing::Protocol(1002), Some((4400, "Invalid message"))),
        (Closing::Protocol(1011), Some((4401, "Unauthorized"))),
        (
            Closing::Protocol(4429),
            Some((4429, "Too many initialisation requests")),
        ),
        (Closing::Protocol(3008), Some((3008, ""))),
    ];
    for (closing, frame) in frames {
        assert_eq!(closing.frame(), frame, "{closing:?}");
    }
}
