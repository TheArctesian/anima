//! The push loop against the fake hub: every K8 Section 6 answer, the rhythm, the buffer.

use std::time::Duration;

use agent_core::wire::RefusalCode;
use anima_agent::push::Event;
use anima_agent::token_store::{Credential, TokenStore};

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{at, device, points, pushed_times, rig, token, year};

fn feed(rig: &mut crate::kit::Rig, first: i64, count: i64) {
    for point in points(first, count) {
        rig.pusher.accept(Event::Point(point));
    }
}

fn seconds(wait: Duration) -> u64 {
    wait.as_secs()
}

#[tokio::test]
async fn k8_7_a_push_sends_oldest_first_at_most_60_points_then_the_next_batch_at_once() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    feed(&mut rig, 1, 130);
    let wait = rig.pusher.push_due(at(1400)).await;
    assert_eq!(seconds(wait), 10);
    let pushes = hub.seen("push");
    let sizes: Vec<usize> = pushes
        .iter()
        .map(|seen| pushed_times(&seen.body).len())
        .collect();
    assert_eq!(sizes, [60, 60, 10]);
    assert_eq!(
        pushed_times(&pushes[0].body)[0],
        10,
        "the oldest point leads"
    );
    assert_eq!(pushed_times(&pushes[2].body).last(), Some(&1300));
    assert_eq!(pushes[0].body["device"], device().to_string());
    assert_eq!(pushes[0].bearer.as_deref(), Some(token(1).expose()));
}

#[tokio::test]
async fn k8_7_nothing_is_sent_when_nothing_waits_and_an_accepted_batch_is_not_sent_again() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    feed(&mut rig, 1, 3);
    rig.pusher.push_due(at(40)).await;
    rig.pusher.push_due(at(50)).await;
    assert_eq!(hub.seen("push").len(), 1);
}

#[tokio::test]
async fn k8_6_malformed_and_out_of_range_drop_that_batch_and_send_the_rest() {
    for code in [RefusalCode::Malformed, RefusalCode::OutOfRange] {
        let hub = FakeHub::start().await;
        let mut rig = rig(&hub, 1, year());
        hub.queue_push([Reply::refuse(code)]);
        feed(&mut rig, 1, 3);
        rig.pusher.push_due(at(40)).await;
        feed(&mut rig, 4, 2);
        rig.pusher.push_due(at(60)).await;
        let pushes = hub.seen("push");
        assert_eq!(pushes.len(), 2, "{code:?}: one dropped, one new");
        assert_eq!(
            pushed_times(&pushes[1].body),
            [40, 50],
            "{code:?}: the dropped points never return"
        );
    }
}

#[tokio::test]
async fn k8_6_too_large_retries_the_same_data_smaller_and_removes_only_what_was_accepted() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::TooLarge)]);
    feed(&mut rig, 1, 4);
    rig.pusher.push_due(at(50)).await;
    let times: Vec<Vec<i64>> = hub
        .seen("push")
        .iter()
        .map(|seen| pushed_times(&seen.body))
        .collect();
    assert_eq!(times, [vec![10, 20, 30, 40], vec![10, 20], vec![30, 40]]);
}

#[tokio::test]
async fn k8_6_one_point_that_is_too_large_is_dropped() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([
        Reply::refuse(RefusalCode::TooLarge),
        Reply::refuse(RefusalCode::TooLarge),
    ]);
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    let times: Vec<Vec<i64>> = hub
        .seen("push")
        .iter()
        .map(|seen| pushed_times(&seen.body))
        .collect();
    assert_eq!(
        times,
        [vec![10, 20], vec![10], vec![20]],
        "the first point is dropped, the second goes"
    );
}

#[tokio::test]
async fn k8_6_a_refused_token_stops_pushing_keeps_buffering_and_retries_in_10_minutes() {
    for code in [RefusalCode::TokenRefused, RefusalCode::WrongDevice] {
        let hub = FakeHub::start().await;
        let mut rig = rig(&hub, 1, year());
        hub.queue_push([Reply::refuse(code)]);
        feed(&mut rig, 1, 2);
        rig.pusher.push_due(at(30)).await;
        assert!(rig.pusher.is_stopped(), "{code:?}");
        feed(&mut rig, 3, 2);
        rig.pusher.push_due(at(60)).await;
        rig.pusher.push_due(at(300)).await;
        assert_eq!(
            hub.seen("push").len(),
            1,
            "{code:?}: no request while stopped"
        );
        rig.pusher.push_due(at(631)).await;
        let pushes = hub.seen("push");
        assert_eq!(pushes.len(), 2, "{code:?}");
        assert_eq!(
            pushed_times(&pushes[1].body),
            [10, 20, 30, 40],
            "{code:?}: the buffer kept every point"
        );
        assert!(!rig.pusher.is_stopped());
    }
}

#[tokio::test]
async fn k8_6_a_changed_token_store_ends_the_stop_at_once() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::TokenRefused)]);
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    // The operator enrols again: another process stores a new token.
    let fresh = Credential {
        token: token(9),
        expires_at: at(0) + year(),
    };
    TokenStore::open(&rig.home).save(&fresh).expect("stored");
    rig.pusher.push_due(at(40)).await;
    let pushes = hub.seen("push");
    assert_eq!(pushes.len(), 2);
    assert_eq!(pushes[1].bearer.as_deref(), Some(token(9).expose()));
}

#[tokio::test]
async fn k8_7_after_a_failure_the_wait_is_10_seconds_doubling_to_60_then_the_rhythm_returns() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    let unavailable = || Reply::refuse(RefusalCode::Unavailable);
    hub.queue_push([
        unavailable(),
        Reply::json(500, ""),
        unavailable(),
        unavailable(),
        unavailable(),
    ]);
    feed(&mut rig, 1, 2);
    let mut waits = Vec::new();
    for round in 0..5 {
        waits.push(seconds(rig.pusher.push_due(at(30 + round * 10)).await));
    }
    assert_eq!(waits, [10, 20, 40, 60, 60]);
    assert_eq!(
        seconds(rig.pusher.push_due(at(200)).await),
        10,
        "accepted: the rhythm"
    );
}

#[tokio::test]
async fn k8_7_a_network_error_keeps_the_points_and_backs_off() {
    // A port nothing listens on.
    let dead = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listener.local_addr().expect("addr");
        drop(listener);
        address
    };
    let dir = tempfile::tempdir().expect("a folder");
    let home = anima_agent::config::home::Home::at(dir.path().to_path_buf());
    let credential = Credential {
        token: token(1),
        expires_at: at(0) + year(),
    };
    let client = anima_agent::hub::HubClient::new(format!("http://{dead}").parse().expect("url"))
        .expect("client");
    let mut pusher =
        anima_agent::push::Pusher::new(client, device(), credential, TokenStore::open(&home));
    for point in points(1, 2) {
        pusher.accept(Event::Point(point));
    }
    assert_eq!(seconds(pusher.push_due(at(30)).await), 10);
    assert_eq!(seconds(pusher.push_due(at(40)).await), 20);
}

#[tokio::test]
async fn met_014_points_kept_through_an_outage_go_out_oldest_first_when_the_hub_returns() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push(std::iter::repeat_n(
        Reply::refuse(RefusalCode::Unavailable),
        2,
    ));
    feed(&mut rig, 1, 3);
    rig.pusher.push_due(at(40)).await;
    feed(&mut rig, 4, 3);
    rig.pusher.push_due(at(70)).await;
    feed(&mut rig, 7, 3);
    rig.pusher.push_due(at(100)).await;
    let last = hub.seen("push").pop().expect("a push");
    assert_eq!(
        pushed_times(&last.body),
        [10, 20, 30, 40, 50, 60, 70, 80, 90]
    );
}

#[tokio::test]
async fn met_014_an_outage_keeps_one_hour_of_points_the_oldest_dropped_first() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::Unavailable)]);
    feed(&mut rig, 1, 400);
    // 400 points are made, the clock reads 3950 s: the cap keeps 360, which start at 410 s.
    rig.pusher.push_due(at(3950)).await;
    rig.pusher.push_due(at(3950)).await;
    let times: Vec<i64> = hub
        .seen("push")
        .iter()
        .skip(1)
        .flat_map(|seen| pushed_times(&seen.body))
        .collect();
    assert_eq!(times.len(), 360);
    assert_eq!(times[0], 410, "the 40 oldest are gone");
    assert_eq!(times.last(), Some(&4000));
}

#[tokio::test]
async fn met_014_a_point_older_than_59_minutes_is_dropped_before_a_send() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::Unavailable)]);
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3600, 1);
    // Ten hours after the first two points, 10 s after the third.
    rig.pusher.push_due(at(36_000)).await;
    let last = hub.seen("push").pop().expect("a push");
    assert_eq!(pushed_times(&last.body), [36_000]);
}
