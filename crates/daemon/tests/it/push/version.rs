//! R-122: each batch the daemon pushes carries the version of the daemon.

use super::feed;
use crate::fake_hub::FakeHub;
use crate::kit::{at, rig, year};

#[tokio::test]
async fn r_122_each_batch_carries_the_version_of_the_daemon() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    let versions: Vec<_> = hub
        .seen("push")
        .iter()
        .map(|seen| seen.body.get("daemonVersion").cloned())
        .collect();
    let version = serde_json::json!(env!("CARGO_PKG_VERSION"));
    assert_eq!(versions, [Some(version.clone()), Some(version)]);
}

/// HMD-055: the desktop app reads this line to compare the daemon it carries with the copy it
/// installed; the program needs no home folder, no token and no network for it.
#[test]
fn hmd_055_the_program_prints_the_same_version_with_an_empty_environment() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_anima-daemon"))
        .arg("--version")
        .env_clear()
        .output()
        .expect("the daemon runs");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("anima-daemon {}\n", env!("CARGO_PKG_VERSION"))
    );
}
