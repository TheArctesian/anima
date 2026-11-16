use std::time::Duration;

use super::{Backoff, pause_after};
use crate::group::LIMITS;
use crate::members;

const fn seconds(count: u64) -> Duration {
    Duration::from_secs(count)
}

#[test]
fn f_6_the_link_ready_line_is_the_slog_message_and_nothing_else() {
    let ready = r#"time=2026-11-15T10:00:00.000+01:00 level=INFO msg="the hub link is ready" relay=relay.example devices=2"#;

    assert_eq!(members::link_is_ready(ready), Some(()));
    for other in [
        r#"time=2026-11-15T10:00:00Z level=INFO msg="the hub link stopped""#,
        r#"time=2026-11-15T10:00:00Z level=INFO msg="relay" note="the hub link is ready""#,
        "the hub link is ready",
    ] {
        assert_eq!(members::link_is_ready(other), None, "{other}");
    }
}

#[test]
fn f_6_the_pause_doubles_from_10_s_to_at_most_60_s() {
    let backoff = LIMITS.link_backoff;
    let mut pause = None;
    let mut pauses = Vec::new();
    for _ in 0..6 {
        let next = pause_after(pause, Duration::ZERO, &backoff);
        pauses.push(next.as_secs());
        pause = Some(next);
    }

    assert_eq!(pauses, [10, 20, 40, 60, 60, 60]);
}

#[test]
fn f_6_a_link_that_was_up_for_5_minutes_starts_again_after_the_first_pause() {
    let backoff = LIMITS.link_backoff;

    assert_eq!(
        pause_after(Some(seconds(60)), seconds(300), &backoff),
        seconds(10)
    );
    assert_eq!(
        pause_after(Some(seconds(60)), seconds(299), &backoff),
        seconds(60)
    );
    assert_eq!(
        pause_after(Some(seconds(20)), seconds(299), &backoff),
        seconds(40)
    );
}

#[test]
fn a_first_pause_above_the_longest_is_the_longest() {
    let backoff = Backoff {
        first: seconds(90),
        longest: seconds(60),
        steady: seconds(300),
    };

    assert_eq!(pause_after(None, Duration::ZERO, &backoff), seconds(60));
}
