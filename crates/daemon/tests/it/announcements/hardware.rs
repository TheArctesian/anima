//! The hardware values in a batch: sent once after they changed, kept until a 204, as the K8
//! JSON of the optional member `hardware` (CAP-032, R-165).

use anima_daemon::push::Event;
use daemon_core::hardware::{
    Backend, BackendName, BackendReason, Hardware, PowerSource, ThermalState, UsableMemory,
    UsableSource,
};

use crate::fake_hub::FakeHub;
use crate::kit::{at, points, rig, year};

const GIB: u64 = 1024 * 1024 * 1024;

fn linux_pc() -> Hardware {
    Hardware {
        system_memory_bytes: 64 * GIB,
        unified_memory_bytes: None,
        dedicated_memory_bytes: Some(24 * GIB),
        usable_memory: None,
        free_disk_bytes: Some(512 * GIB),
        cpu_features: vec!["avx2".to_owned()],
        backends: vec![
            Backend {
                name: BackendName::Cuda,
                state: None,
            },
            Backend {
                name: BackendName::Rocm,
                state: Some(BackendReason::NoDevice),
            },
        ],
        power_source: Some(PowerSource::Mains),
        thermal_state: None,
        operator_active: None,
    }
}

#[tokio::test]
async fn cap_032_the_hardware_goes_in_the_next_batch_and_not_in_the_one_after() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    rig.pusher.accept(Event::Hardware(linux_pc()));
    for point in points(1, 2) {
        rig.pusher.accept(Event::Point(point));
    }
    rig.pusher.push_due(at(30)).await;
    for point in points(3, 2) {
        rig.pusher.accept(Event::Point(point));
    }
    rig.pusher.push_due(at(50)).await;
    let sent: Vec<Option<serde_json::Value>> = hub
        .seen("push")
        .iter()
        .map(|seen| seen.body.get("hardware").cloned())
        .collect();
    let expected = serde_json::json!({
        "systemMemoryBytes": 64 * GIB, "dedicatedMemoryBytes": 24 * GIB,
        "freeDiskBytes": 512 * GIB, "cpuFeatures": ["avx2"],
        "backends": [{ "name": "cuda" }, { "name": "rocm", "state": "no_device" }],
        "powerSource": "mains"
    });
    assert_eq!(sent, [Some(expected), None], "cleared after the 204");
}

#[test]
fn cap_005_a_mac_sends_its_usable_memory_with_its_source() {
    let mac = Hardware {
        unified_memory_bytes: Some(48 * GIB),
        dedicated_memory_bytes: None,
        usable_memory: Some(UsableMemory {
            bytes: 36 * GIB,
            source: UsableSource::DefaultShare,
        }),
        thermal_state: Some(ThermalState::Nominal),
        operator_active: Some(false),
        ..linux_pc()
    };
    let text = serde_json::to_value(&mac).expect("json");
    assert_eq!(text["usableMemory"]["source"], "default_share");
    assert_eq!(text["thermalState"], "nominal");
    assert_eq!(text["operatorActive"], false);
}
