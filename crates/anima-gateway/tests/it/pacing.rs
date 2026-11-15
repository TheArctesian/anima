//! Frames go out only as fast as the limit allows and the client draws them.

use std::time::Duration;

use anima_gateway::frames::{Formats, Next, Pace, Pacing, frame};
use anima_gateway::screen::{Area, Screen};
use proptest::prelude::*;

use crate::drawn::Drawn;
use crate::screen::whole;

const PACE: Pace = Pace {
    least_interval: Duration::from_millis(20),
    most_unanswered: 2,
};

const fn at(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn the_first_frame_goes_at_once() {
    assert_eq!(Pacing::new(PACE).next(at(0)), Next::Now);
}

#[test]
fn the_next_frame_waits_for_the_frame_rate_limit() {
    let mut pacing = Pacing::new(PACE);
    pacing.sent(at(100));
    assert_eq!(pacing.next(at(105)), Next::At(at(120)));
    assert_eq!(pacing.next(at(120)), Next::Now);
}

#[test]
fn a_client_that_is_behind_gets_no_frame_however_long_it_takes() {
    let mut pacing = Pacing::new(PACE);
    pacing.sent(at(0));
    pacing.sent(at(20));
    assert_eq!(pacing.next(at(60_000)), Next::AfterAnswer);
}

#[test]
fn an_answer_lets_the_next_frame_go() {
    let mut pacing = Pacing::new(PACE);
    let first = pacing.sent(at(0));
    pacing.sent(at(20));
    pacing.answered(&first.to_string());
    assert_eq!(pacing.next(at(40)), Next::Now);
}

#[test]
fn an_answer_for_a_frame_answers_the_frames_before_it() {
    let mut pacing = Pacing::new(Pace {
        most_unanswered: 3,
        ..PACE
    });
    pacing.sent(at(0));
    pacing.sent(at(20));
    let third = pacing.sent(at(40));
    pacing.answered(&third.to_string());
    pacing.sent(at(60));
    pacing.sent(at(80));
    assert_eq!(pacing.next(at(100)), Next::Now);
}

#[test]
fn text_that_is_not_a_timestamp_answers_nothing() {
    let mut pacing = Pacing::new(PACE);
    pacing.sent(at(0));
    pacing.sent(at(20));
    for answer in ["", "soon", "-1", "1.5", "99999999999999999999999"] {
        pacing.answered(answer);
    }
    assert_eq!(pacing.next(at(40)), Next::AfterAnswer);
}

#[test]
fn frames_sent_in_the_same_millisecond_get_rising_timestamps() {
    let mut pacing = Pacing::new(Pace {
        least_interval: Duration::ZERO,
        most_unanswered: 8,
    });
    let stamps: Vec<u64> = (0..4).map(|_| pacing.sent(at(7))).collect();
    assert_eq!(stamps, [7, 8, 9, 10]);
}

/// A device that changes its screen every 10 ms, and a client that takes `draw` ms to
/// answer each frame. Returns the frames sent and whether the client ends with the screen.
fn session(draw: u64) -> (usize, bool) {
    let mut screen = Screen::new(64, 64).expect("a valid size");
    let mut pacing = Pacing::new(PACE);
    let mut client = Drawn::default();
    let mut answers: Vec<(u64, u64)> = Vec::new();
    let all = Area {
        x: 0,
        y: 0,
        width: 64,
        height: 64,
    };
    let mut frames = 0;
    for now in 0..=2000u64 {
        answers.retain(|(due, timestamp)| {
            let due_now = *due <= now;
            if due_now {
                pacing.answered(&timestamp.to_string());
            }
            !due_now
        });
        if now % 10 == 0 && now <= 1000 {
            let colour = [now.to_le_bytes()[0], now.to_le_bytes()[1], 1];
            screen.write(all, &colour.repeat(64 * 64)).expect("inside");
        }
        if pacing.next(at(now)) == Next::Now {
            let changes = screen.take_changes();
            if changes.resized || !changes.moved.is_empty() || !changes.areas.is_empty() {
                let timestamp = pacing.sent(at(now));
                client.receive(
                    &frame(&screen, &changes, Formats::default(), timestamp).expect("a frame"),
                );
                answers.push((now + draw, timestamp));
                frames += 1;
            }
        }
    }
    (frames, client.rgb == whole(&screen))
}

#[test]
fn a_slow_client_gets_fewer_frames_and_still_ends_with_the_screen() {
    let (fast_frames, fast_shows_it) = session(1);
    let (slow_frames, slow_shows_it) = session(200);
    assert!(fast_shows_it && slow_shows_it);
    // The device changed 101 times. The limit is 50 frames each second; the slow client
    // answers 5 frames each second, two at a time.
    assert!(
        (40..=52).contains(&fast_frames),
        "{fast_frames} frames to the fast client"
    );
    assert!(slow_frames <= 14, "{slow_frames} frames to the slow client");
}

/// What can happen to the pacing of a session.
#[derive(Clone, Debug)]
enum Event {
    /// Time passes, then a frame is sent if one may be.
    Tick(u64),
    /// The client answers the frame sent this many frames ago, or sends other text.
    Answer(usize),
    Noise(String),
}

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![
        4 => (0..60u64).prop_map(Event::Tick),
        2 => (0..4usize).prop_map(Event::Answer),
        1 => "[0-9a-z.-]{0,12}".prop_map(Event::Noise),
    ]
}

proptest! {
    /// Whatever the client does: never more unanswered frames than the limit, never two
    /// frames closer than the interval, and timestamps always rise.
    #[test]
    fn the_limits_hold_for_any_client(events in prop::collection::vec(event(), 0..200)) {
        let mut pacing = Pacing::new(PACE);
        let (mut now, mut sent, mut answered_up_to) = (0u64, Vec::<(u64, u64)>::new(), 0u64);
        for event in events {
            match event {
                Event::Tick(millis) => {
                    now += millis;
                    if pacing.next(at(now)) == Next::Now {
                        let timestamp = pacing.sent(at(now));
                        if let Some((before_at, before)) = sent.last() {
                            prop_assert!(timestamp > *before);
                            prop_assert!(now - before_at >= 20);
                        }
                        sent.push((now, timestamp));
                    }
                }
                Event::Answer(ago) => {
                    if let Some((_, timestamp)) = sent.iter().rev().nth(ago) {
                        pacing.answered(&timestamp.to_string());
                        answered_up_to = answered_up_to.max(*timestamp);
                    }
                }
                Event::Noise(text) => {
                    if text.parse::<u64>().is_err() {
                        pacing.answered(&text);
                    }
                }
            }
            let unanswered = sent.iter().filter(|(_, timestamp)| *timestamp > answered_up_to).count();
            prop_assert!(unanswered <= PACE.most_unanswered, "{unanswered} unanswered frames");
        }
    }
}
