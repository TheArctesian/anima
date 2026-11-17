//! The tool inventory in a batch: sent once after it changed, kept until a 204, dropped after a
//! `MALFORMED` (CAP-027, CAP-029, K8 Section 4).

use agent_core::inventory::{Architecture, Os};
use agent_core::wire::{BatchInventory, BatchTool, RefusalCode};
use anima_agent::push::Event;

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{Rig, at, points, rig, year};

fn feed(rig: &mut Rig, first: i64, count: i64) {
    for point in points(first, count) {
        rig.pusher.accept(Event::Point(point));
    }
}

fn mac() -> BatchInventory {
    BatchInventory {
        architecture: Architecture::Aarch64,
        os: Os::Macos,
        os_version: "27.0".to_owned(),
        tools: vec![BatchTool {
            name: "cargo".to_owned(),
            version: Some("1.91.0".to_owned()),
            state: None,
        }],
        build_targets: vec!["aarch64-apple-darwin".to_owned()],
    }
}

fn sent() -> serde_json::Value {
    serde_json::json!({
        "architecture": "aarch64", "os": "macos", "osVersion": "27.0",
        "tools": [{ "name": "cargo", "version": "1.91.0" }],
        "buildTargets": ["aarch64-apple-darwin"]
    })
}

fn inventories_of(hub: &FakeHub) -> Vec<Option<serde_json::Value>> {
    hub.seen("push")
        .iter()
        .map(|seen| seen.body.get("inventory").cloned())
        .collect()
}

#[tokio::test]
async fn cap_027_the_inventory_goes_in_the_next_batch_and_not_in_the_one_after() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    rig.pusher.accept(Event::Inventory(mac()));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    assert_eq!(
        inventories_of(&hub),
        [Some(sent()), None],
        "cleared after the 204"
    );
}

#[tokio::test]
async fn cap_027_the_inventory_is_sent_again_until_a_batch_is_accepted() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::OutOfRange), Reply::json(500, "")]);
    rig.pusher.accept(Event::Inventory(mac()));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 1);
    rig.pusher.push_due(at(40)).await;
    rig.pusher.push_due(at(50)).await;
    feed(&mut rig, 4, 1);
    rig.pusher.push_due(at(60)).await;
    assert_eq!(
        inventories_of(&hub),
        [Some(sent()), Some(sent()), Some(sent()), None],
        "kept through a dropped batch and a 5xx, cleared after the 204"
    );
}

#[tokio::test]
async fn cap_029_a_refused_inventory_is_not_sent_again_after_malformed() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::Malformed)]);
    rig.pusher.accept(Event::Inventory(mac()));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    assert_eq!(inventories_of(&hub), [Some(sent()), None]);
}
