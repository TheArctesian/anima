//! The GPU list in a batch: sent once after a discovery, kept until a 204 or a `MALFORMED`
//! (K8 Section 4, R-098).

use agent_core::wire::RefusalCode;
use anima_agent::push::Event;

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{at, points, rig, year};

fn feed(rig: &mut crate::kit::Rig, first: i64, count: i64) {
    for point in points(first, count) {
        rig.pusher.accept(Event::Point(point));
    }
}

fn gpu_list() -> Vec<agent_core::wire::BatchGpu> {
    vec![agent_core::wire::BatchGpu {
        index: 0,
        name: "RTX A5000".to_owned(),
        vendor: agent_core::wire::GpuVendor::Nvidia,
        has_vram: true,
    }]
}

fn gpus_of(hub: &FakeHub) -> Vec<Option<serde_json::Value>> {
    hub.seen("push")
        .iter()
        .map(|seen| seen.body.get("gpus").cloned())
        .collect()
}

fn expected_gpus() -> serde_json::Value {
    serde_json::json!([{ "index": 0, "name": "RTX A5000", "vendor": "nvidia", "hasVram": true }])
}

#[tokio::test]
async fn k8_4_the_gpu_list_goes_in_the_first_batch_after_a_discovery_and_not_in_the_next() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    rig.pusher.accept(Event::Gpus(gpu_list()));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    assert_eq!(
        gpus_of(&hub),
        [Some(expected_gpus()), None],
        "cleared after the 204"
    );
}

#[tokio::test]
async fn k8_4_a_discovery_that_found_no_gpu_sends_an_empty_list_once() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    rig.pusher.accept(Event::Gpus(Vec::new()));
    feed(&mut rig, 1, 1);
    rig.pusher.push_due(at(20)).await;
    feed(&mut rig, 2, 1);
    rig.pusher.push_due(at(30)).await;
    assert_eq!(gpus_of(&hub), [Some(serde_json::json!([])), None]);
}

#[tokio::test]
async fn k8_4_the_gpu_list_is_kept_when_the_hub_drops_the_batch_for_its_times() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::OutOfRange)]);
    rig.pusher.accept(Event::Gpus(gpu_list()));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 1);
    rig.pusher.push_due(at(40)).await;
    assert_eq!(
        gpus_of(&hub),
        [Some(expected_gpus()), Some(expected_gpus())]
    );
}

/// T8 N1: a list the hub refuses (index 64 is past K8's 63) is not sent again, so it cannot
/// sink every later batch until the next discovery, 24 hours on.
#[tokio::test]
async fn k8_4_a_refused_gpu_list_is_not_resent_with_every_batch() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([
        Reply::refuse(RefusalCode::Malformed),
        Reply::refuse(RefusalCode::Malformed),
    ]);
    rig.pusher
        .accept(Event::Gpus(vec![agent_core::wire::BatchGpu {
            index: 64,
            name: "GPU".to_owned(),
            vendor: agent_core::wire::GpuVendor::Nvidia,
            has_vram: true,
        }]));
    feed(&mut rig, 1, 3);
    rig.pusher.push_due(at(40)).await;
    feed(&mut rig, 4, 2);
    rig.pusher.push_due(at(60)).await;
    let pushes = gpus_of(&hub);
    assert_eq!(pushes.len(), 2);
    assert_eq!(
        pushes[1], None,
        "the refused gpus list rode along again, so this batch was dropped too"
    );
}

#[tokio::test]
async fn k8_4_the_gpu_list_is_kept_through_a_5xx_and_a_503() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([
        Reply::json(500, ""),
        Reply::refuse(RefusalCode::Unavailable),
    ]);
    rig.pusher.accept(Event::Gpus(gpu_list()));
    feed(&mut rig, 1, 2);
    for round in 0..3 {
        rig.pusher.push_due(at(30 + round * 10)).await;
    }
    assert_eq!(
        gpus_of(&hub),
        [
            Some(expected_gpus()),
            Some(expected_gpus()),
            Some(expected_gpus())
        ]
    );
    rig.pusher.push_due(at(70)).await;
    assert_eq!(
        hub.seen("push").len(),
        3,
        "the third try was accepted and nothing waits"
    );
}

#[tokio::test]
async fn k8_4_the_gpu_list_is_sent_again_with_a_smaller_batch_after_too_large() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::TooLarge)]);
    rig.pusher.accept(Event::Gpus(gpu_list()));
    feed(&mut rig, 1, 4);
    rig.pusher.push_due(at(50)).await;
    assert_eq!(
        gpus_of(&hub),
        [Some(expected_gpus()), Some(expected_gpus()), None]
    );
}
