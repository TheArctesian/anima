//! The drive list in a batch: sent once after it changed, kept until a 204 or a `MALFORMED`
//! (MET-022, K8 Section 4).

use agent_core::wire::{BatchDrive, DriveKind};
use anima_agent::push::Event;

use crate::fake_hub::FakeHub;
use crate::kit::{at, points, rig, year};

fn feed(rig: &mut crate::kit::Rig, first: i64, count: i64) {
    for point in points(first, count) {
        rig.pusher.accept(Event::Point(point));
    }
}

fn drives_of(hub: &FakeHub) -> Vec<Option<serde_json::Value>> {
    hub.seen("push")
        .iter()
        .map(|seen| seen.body.get("drives").cloned())
        .collect()
}

#[tokio::test]
async fn met_022_the_drive_list_goes_in_the_next_batch_and_not_in_the_one_after() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    rig.pusher.accept(Event::Drives(vec![BatchDrive {
        name: "nvme1n1".to_owned(),
        kind: DriveKind::Nvme,
        size_bytes: 1_000_204_886_016,
        mounts: vec!["/".to_owned(), "/boot/efi".to_owned()],
    }]));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    let expected = serde_json::json!([{
        "name": "nvme1n1", "kind": "nvme", "sizeBytes": 1_000_204_886_016_u64,
        "mounts": ["/", "/boot/efi"]
    }]);
    assert_eq!(
        drives_of(&hub),
        [Some(expected), None],
        "cleared after the 204"
    );
}
