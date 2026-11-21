//! The battery list in a batch: sent once after it changed, kept until a 204, and not sent
//! again after a `MALFORMED` (MET-032, K8 Section 4).

use anima_daemon::push::Event;
use daemon_core::wire::{BatchBattery, BatteryState, RefusalCode};

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{at, points, rig, year};

fn feed(rig: &mut crate::kit::Rig, first: i64, count: i64) {
    for point in points(first, count) {
        rig.pusher.accept(Event::Point(point));
    }
}

fn batteries_of(hub: &FakeHub) -> Vec<Option<serde_json::Value>> {
    hub.seen("push")
        .iter()
        .map(|seen| seen.body.get("batteries").cloned())
        .collect()
}

fn laptop() -> Vec<BatchBattery> {
    vec![BatchBattery {
        name: "BAT0".to_owned(),
        state: BatteryState::Discharging,
        seconds_left: Some(9000),
        health_percent: Some(91),
    }]
}

#[tokio::test]
async fn met_032_the_battery_list_goes_in_the_next_batch_and_is_kept_until_one_is_accepted() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::json(500, "")]);
    rig.pusher.accept(Event::Batteries(laptop()));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    rig.pusher.push_due(at(40)).await;
    feed(&mut rig, 3, 1);
    rig.pusher.push_due(at(50)).await;
    let expected = serde_json::json!([{
        "name": "BAT0", "state": "discharging", "secondsLeft": 9000, "healthPercent": 91
    }]);
    assert_eq!(
        batteries_of(&hub),
        [Some(expected.clone()), Some(expected), None],
        "sent again after the 500, cleared after the 204"
    );
}

#[tokio::test]
async fn met_032_a_refused_battery_list_waits_for_the_next_change() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::Malformed)]);
    rig.pusher.accept(Event::Batteries(laptop()));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 1);
    rig.pusher.push_due(at(40)).await;
    let pushes = batteries_of(&hub);
    assert_eq!(pushes.len(), 2);
    assert_eq!(pushes[1], None, "the refused list did not ride along again");
}
