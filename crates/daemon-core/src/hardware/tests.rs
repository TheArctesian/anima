//! Tests of the backend verdicts for a Mac, an NVIDIA Linux PC and a machine with no GPU, of
//! the parse of the readers' output, and of the check of the batch member.

use super::parse::{ioreg_active, pci_vendor, pmset_thermal, sysctl_number, usable_memory};
use super::power::{Power, Supply, from_pmset, from_supplies};
use super::*;
use crate::inventory::Os;
use crate::readings::GpuVendor;

const GIB: u64 = 1024 * 1024 * 1024;

fn facts(os: Os, vendors: Vec<GpuVendor>) -> Facts {
    Facts {
        os,
        apple_silicon: false,
        vendors,
        nvidia_driver: false,
        kfd: KfdAccess::Missing,
        vulkan_driver: false,
    }
}

fn states(found: &[Backend]) -> Vec<(&'static str, Option<BackendReason>)> {
    found
        .iter()
        .map(|one| (one.name.as_str(), one.state))
        .collect()
}

#[test]
fn cap_002_an_apple_silicon_mac_has_metal_and_the_cpu_only() {
    let mac = Facts {
        apple_silicon: true,
        ..facts(Os::Macos, vec![])
    };
    let unsupported = Some(BackendReason::NotSupported);
    assert_eq!(
        states(&backends(&mac)),
        [
            ("metal", None),
            ("cuda", unsupported),
            ("rocm", unsupported),
            ("vulkan", unsupported),
            ("cpu", None),
        ]
    );
}

#[test]
fn cap_002_an_nvidia_linux_pc_has_cuda_and_vulkan_when_the_drivers_are_there() {
    let pc = Facts {
        nvidia_driver: true,
        vulkan_driver: true,
        ..facts(Os::Linux, vec![GpuVendor::Nvidia])
    };
    assert_eq!(
        states(&backends(&pc)),
        [
            ("metal", Some(BackendReason::NotSupported)),
            ("cuda", None),
            ("rocm", Some(BackendReason::NoDevice)),
            ("vulkan", None),
            ("cpu", None),
        ]
    );
    let no_driver = facts(Os::Linux, vec![GpuVendor::Nvidia]);
    let found = backends(&no_driver);
    assert_eq!(found[1].state, Some(BackendReason::NoDriver));
    assert_eq!(found[3].state, Some(BackendReason::NoDriver));
}

#[test]
fn cap_002_a_machine_with_no_gpu_has_the_cpu_only() {
    let found = backends(&facts(Os::Linux, vec![]));
    assert!(found[4].is_usable());
    assert!(
        found[1..4]
            .iter()
            .all(|one| one.state == Some(BackendReason::NoDevice))
    );
}

#[test]
fn cap_002_rocm_says_whether_the_driver_or_the_permission_is_missing() {
    let amd = |kfd| Facts {
        kfd,
        ..facts(Os::Linux, vec![GpuVendor::Amd])
    };
    let rocm = |kfd| backends(&amd(kfd))[2].state;
    assert_eq!(rocm(KfdAccess::Missing), Some(BackendReason::NoDriver));
    assert_eq!(rocm(KfdAccess::Denied), Some(BackendReason::NoPermission));
    assert_eq!(rocm(KfdAccess::Granted), None);
}

#[test]
fn cap_005_the_usable_memory_is_the_wired_limit_or_the_default_share() {
    let set = usable_memory(48 * GIB, Some(40_000));
    assert_eq!(set.bytes, 40_000 * 1024 * 1024);
    assert_eq!(set.source, UsableSource::WiredLimit);
    let large = usable_memory(48 * GIB, Some(0));
    assert_eq!(
        (large.bytes, large.source),
        (36 * GIB, UsableSource::DefaultShare)
    );
    let small = usable_memory(24 * GIB, None);
    assert_eq!(small.bytes, 16 * GIB);
    assert_eq!(
        usable_memory(8 * GIB, Some(999_999)).bytes,
        8 * GIB,
        "never past memory"
    );
}

#[test]
fn cap_001_sysctl_numbers_are_read_and_garbage_is_none() {
    assert_eq!(sysctl_number(b"51539607552\n"), Some(51_539_607_552));
    assert_eq!(sysctl_number(b"0\n"), Some(0));
    assert_eq!(sysctl_number(b"unknown oid\n"), None);
    assert_eq!(sysctl_number(b""), None);
}

#[test]
fn cap_004_the_thermal_state_of_pmset() {
    let quiet = b"Note: No thermal warning level has been recorded\nNote: No performance warning level has been recorded\n";
    assert_eq!(pmset_thermal(quiet), Some(ThermalState::Nominal));
    let hot = b"CPU_Scheduler_Limit \t= 100\nCPU_Available_CPUs \t= 12\nCPU_Speed_Limit \t= 70\n";
    assert_eq!(pmset_thermal(hot), Some(ThermalState::Throttled));
    assert_eq!(
        pmset_thermal(b"CPU_Speed_Limit = 100\n"),
        Some(ThermalState::Nominal)
    );
    assert_eq!(pmset_thermal(b"garbage"), None);
}

#[test]
fn cap_004_the_operator_is_active_within_five_minutes_of_input() {
    let line = |ns: u64| format!("  |   \"HIDIdleTime\" = {ns}\n");
    assert_eq!(ioreg_active(line(165_115_875).as_bytes()), Some(true));
    assert_eq!(ioreg_active(line(301_000_000_000).as_bytes()), Some(false));
    assert_eq!(ioreg_active(b"+-o IOHIDSystem\n"), None);
}

#[test]
fn cap_004_the_power_source_of_a_mac_and_of_a_linux_laptop() {
    let mains = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t93%; charging; 0:45 remaining present: true\n";
    let battery =
        "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t37%; discharging\n";
    assert_eq!(from_pmset(mains), Power::Mains);
    assert_eq!(from_pmset(battery), Power::Battery { percent: 37 });
    assert_eq!(PowerSource::from(from_pmset(battery)), PowerSource::Battery);
    let supply = |scope: &str| Supply {
        kind: "Battery".into(),
        scope: scope.into(),
        status: "Discharging".into(),
        capacity: Some(41),
    };
    assert_eq!(
        from_supplies(&[supply("System")]),
        Power::Battery { percent: 41 }
    );
    assert_eq!(from_supplies(&[supply("Device")]), Power::Mains, "a mouse");
}

#[test]
fn cap_002_pci_vendors_name_the_gpu_vendor() {
    assert_eq!(pci_vendor("0x10de\n"), Some(GpuVendor::Nvidia));
    assert_eq!(pci_vendor("0x1002"), Some(GpuVendor::Amd));
    assert_eq!(pci_vendor("0x1234"), None);
}

fn mac() -> Hardware {
    Hardware {
        system_memory_bytes: 48 * GIB,
        unified_memory_bytes: Some(48 * GIB),
        dedicated_memory_bytes: None,
        usable_memory: Some(usable_memory(48 * GIB, None)),
        free_disk_bytes: Some(400 * GIB),
        cpu_features: vec!["neon".into(), "dotprod".into()],
        backends: backends(&Facts {
            apple_silicon: true,
            ..facts(Os::Macos, vec![])
        }),
        power_source: Some(PowerSource::Mains),
        thermal_state: Some(ThermalState::Nominal),
        operator_active: Some(true),
    }
}

#[test]
fn cap_032_the_member_round_trips_and_its_features_are_sorted() {
    let text = serde_json::to_string(&mac()).unwrap();
    assert!(text.contains("\"usableMemory\":{\"bytes\":38654705664,\"source\":\"default_share\"}"));
    assert!(text.contains("{\"name\":\"cuda\",\"state\":\"not_supported\"}"));
    let back: Hardware = serde_json::from_str(&text).unwrap();
    let accepted = back.accepted().expect("fits");
    assert_eq!(accepted.cpu_features, ["dotprod", "neon"]);
}

#[test]
fn cap_032_names_and_states_a_newer_daemon_knows_are_dropped_or_read_as_failed() {
    let text = serde_json::to_string(&mac())
        .unwrap()
        .replace("\"metal\"", "\"opencl\"")
        .replace("\"nominal\"", "\"hot\"")
        .replace("\"not_supported\"", "\"later\"");
    let accepted = serde_json::from_str::<Hardware>(&text)
        .unwrap()
        .accepted()
        .unwrap();
    assert_eq!(accepted.backends.len(), 4, "the unknown backend is dropped");
    assert_eq!(accepted.backends[0].state, Some(BackendReason::Failed));
    assert_eq!(accepted.thermal_state, None);
}

#[test]
fn cap_032_a_member_that_breaks_the_rules_is_refused() {
    let past = Hardware {
        system_memory_bytes: u64::MAX,
        ..mac()
    };
    assert_eq!(past.accepted(), None);
    let twice = Hardware {
        cpu_features: vec!["neon".into(), "neon".into()],
        ..mac()
    };
    assert_eq!(twice.accepted(), None);
    let shouting = Hardware {
        cpu_features: vec!["NEON".into()],
        ..mac()
    };
    assert_eq!(shouting.accepted(), None);
    let mut doubled = mac();
    doubled.backends.push(doubled.backends[0]);
    assert_eq!(doubled.accepted(), None);
}
