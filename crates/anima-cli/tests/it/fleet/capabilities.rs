//! `anima capability list` against the scripted hub (CAP-030, CMD-020, CMD-030).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const TOKEN: &str = "login-token-for-capability-tests";

/// The K1 `hardware` of an Apple silicon Mac with 48 GiB (CAP-005: the default share).
fn mac_hardware() -> Value {
    json!({ "systemMemoryBytes": 51_539_607_552.0_f64, "unifiedMemoryBytes": 51_539_607_552.0_f64,
            "dedicatedMemoryBytes": null, "usableMemoryBytes": 38_654_705_664.0_f64,
            "usableMemorySource": "DEFAULT_SHARE", "freeDiskBytes": 418_759_311_360.0_f64,
            "cpuFeatures": ["bf16", "dotprod", "fp16", "i8mm", "neon"],
            "backends": [ { "name": "metal", "usable": true, "reason": null },
                          { "name": "cuda", "usable": false, "reason": "NOT_SUPPORTED" },
                          { "name": "rocm", "usable": false, "reason": "NOT_SUPPORTED" },
                          { "name": "vulkan", "usable": false, "reason": "NOT_SUPPORTED" },
                          { "name": "cpu", "usable": true, "reason": null } ],
            "powerSource": "MAINS", "thermalState": "NOMINAL", "operatorActive": true,
            "foundAt": "2026-11-15T09:00:00+00:00" })
}

/// The K1 `hardware` of a Linux PC with one 24 GiB NVIDIA GPU and its drivers.
fn nvidia_hardware() -> Value {
    json!({ "systemMemoryBytes": 68_719_476_736.0_f64, "unifiedMemoryBytes": null,
            "dedicatedMemoryBytes": 25_769_803_776.0_f64, "usableMemoryBytes": null,
            "usableMemorySource": null, "freeDiskBytes": 549_755_813_888.0_f64,
            "cpuFeatures": ["avx", "avx2", "f16c", "fma", "sse4.2"],
            "backends": [ { "name": "metal", "usable": false, "reason": "NOT_SUPPORTED" },
                          { "name": "cuda", "usable": true, "reason": null },
                          { "name": "rocm", "usable": false, "reason": "NO_DEVICE" },
                          { "name": "vulkan", "usable": true, "reason": null },
                          { "name": "cpu", "usable": true, "reason": null } ],
            "powerSource": "MAINS", "thermalState": null, "operatorActive": null,
            "foundAt": "2026-11-15T09:01:00+00:00" })
}

/// The K1 `capabilities` answer: a Mac with cargo and a bun that timed out, and an NVIDIA Linux
/// PC with no tools; each with its hardware values (CAP-032).
pub fn capabilities() -> Value {
    json!({ "capabilities": [
        { "device": { "id": "0190a000-0000-7000-8002-00000000000a", "name": "mac-office-1" },
          "architecture": "aarch64", "os": "macos", "osVersion": "27.0",
          "tools": [ { "name": "cargo", "version": "1.91.0", "usable": true, "reason": null },
                     { "name": "bun", "version": null, "usable": false,
                       "reason": "TIMED_OUT" } ],
          "buildTargets": ["aarch64-apple-darwin", "aarch64-apple-ios"],
          "foundAt": "2026-11-15T09:00:00+00:00", "hardware": mac_hardware() },
        { "device": { "id": "0190a000-0000-7000-8002-000000000009", "name": "linux-pc-home" },
          "architecture": "x86_64", "os": "linux", "osVersion": "24.04",
          "tools": [], "buildTargets": [], "foundAt": "2026-11-15T09:01:00+00:00",
          "hardware": nvidia_hardware() }
    ] })
}

#[tokio::test]
async fn cap_030_capability_list_shows_each_device_s_system_tools_and_targets() {
    let hub = FakeHub::start().await;
    hub.answer("CapabilityList", data(capabilities()));
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&["capability", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert!(lines[0].starts_with("DEVICE"), "{}", run.stdout);
    assert!(
        lines[1].contains("mac-office-1")
            && lines[1].contains("macos 27.0 aarch64")
            && lines[1].contains("cargo 1.91.0, bun not usable (timed out)")
            && lines[1].contains("aarch64-apple-darwin, aarch64-apple-ios"),
        "{}",
        run.stdout
    );
    assert!(
        lines[2].contains("linux-pc-home") && lines[2].contains("linux 24.04 x86_64"),
        "{}",
        run.stdout
    );
    assert!(
        lines[1].contains("48 GiB memory, 48 GiB unified, 36 GiB usable (default share)")
            && lines[1].contains("backends metal, cpu; mains, nominal, active"),
        "{}",
        run.stdout
    );
    assert!(
        lines[2].contains("64 GiB memory, 24 GiB dedicated, 512 GiB free")
            && lines[2].contains("backends cuda, vulkan, cpu; mains"),
        "{}",
        run.stdout
    );
    assert_eq!(
        run.stderr.trim_end(),
        "mac-office-1: bun: Its version check ran past 5 seconds: run it with --version on the \
         device to see what it waits for.",
        "the repair hint goes to standard error"
    );
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(sent.variables["tool"], Value::Null);
    assert_eq!(sent.variables["buildTarget"], Value::Null);
}

#[tokio::test]
async fn cap_030_capability_list_filters_by_tool_and_target_and_prints_k1_json() {
    let hub = FakeHub::start().await;
    hub.answer("CapabilityList", data(capabilities()));
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&[
            "capability",
            "list",
            "--tool",
            "cargo",
            "--target",
            "aarch64-apple-darwin",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), capabilities()["capabilities"]);
    assert_eq!(
        run.stderr, "",
        "no hint with --json: the reason is in the JSON"
    );
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(sent.variables["tool"], "cargo");
    assert_eq!(sent.variables["buildTarget"], "aarch64-apple-darwin");
}

#[tokio::test]
async fn cap_002_a_machine_with_no_gpu_shows_the_cpu_and_a_repairable_backend_gets_a_hint() {
    let hub = FakeHub::start().await;
    let mut answer = capabilities();
    answer["capabilities"]
        .as_array_mut()
        .expect("a list")
        .truncate(1);
    let device = &mut answer["capabilities"][0];
    device["tools"] = json!([]);
    device["hardware"]["backends"] = json!([
        { "name": "rocm", "usable": false, "reason": "NO_PERMISSION" },
        { "name": "vulkan", "usable": false, "reason": "NO_DEVICE" },
        { "name": "cpu", "usable": true, "reason": null } ]);
    hub.answer("CapabilityList", data(answer));
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&["capability", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("backends cpu;"), "{}", run.stdout);
    assert!(
        run.stderr
            .starts_with("mac-office-1: rocm: Add the daemon's user to the render"),
        "only the repairable backend gets a hint: {}",
        run.stderr
    );
    assert_eq!(run.stderr.lines().count(), 1, "{}", run.stderr);
}
