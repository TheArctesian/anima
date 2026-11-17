//! R-122: each batch the agent pushes carries the version of the agent.

use super::feed;
use crate::fake_hub::FakeHub;
use crate::kit::{at, rig, year};

#[tokio::test]
async fn r_122_each_batch_carries_the_version_of_the_agent() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    let versions: Vec<_> = hub
        .seen("push")
        .iter()
        .map(|seen| seen.body.get("agentVersion").cloned())
        .collect();
    let version = serde_json::json!(env!("CARGO_PKG_VERSION"));
    assert_eq!(versions, [Some(version.clone()), Some(version)]);
}
