//! The bodies of the work route read and write as the contract K10 spells them.

use serde_json::json;

use crate::wire::{
    Claim, RefusalCode, Reported, StopReason, Switch, WorkJob, WorkKind, WorkReport, WorkState,
    WorkType,
};

#[test]
fn k10_4_the_claim_of_the_contract_reads() {
    let claim: Claim = serde_json::from_value(json!({
        "device": "0192f5c4-6f1e-7a3b-9c2d-5e8f1a2b3c4d",
        "switch": "on",
        "types": ["build", "runner"],
        "kinds": ["container"],
        "os": "linux",
        "architecture": "x86_64",
        "free": { "slots": 1, "memoryBytes": 17_179_869_184_u64, "diskBytes": 85_899_345_920_u64 },
        "holds": [ { "id": "0192f6aa-0000-7000-8000-000000000001", "attempt": 1 } ],
        "unclean": false
    }))
    .unwrap();
    assert_eq!(claim.switch, Switch::On);
    assert_eq!(claim.types, [WorkType::Build, WorkType::Runner]);
    assert_eq!(claim.kinds, [WorkKind::Container]);
    assert_eq!((claim.free.slots, claim.holds[0].attempt), (1, 1));
}

#[test]
fn k10_4_a_claim_names_a_type_that_the_hub_does_not_know_and_still_reads() {
    let claim: Claim = serde_json::from_value(json!({
        "device": "0192f5c4-6f1e-7a3b-9c2d-5e8f1a2b3c4d", "switch": "off",
        "types": ["build", "model_run"], "kinds": ["container", "microvm"],
        "os": "macos", "architecture": "aarch64",
        "free": { "slots": 0, "memoryBytes": 0, "diskBytes": 0 }
    }))
    .unwrap();
    assert_eq!(claim.types, [WorkType::Build, WorkType::Unknown]);
    assert_eq!(claim.kinds, [WorkKind::Container, WorkKind::Unknown]);
    assert!(claim.holds.is_empty() && !claim.unclean);
}

#[test]
fn k10_4_a_job_is_written_with_the_member_names_of_the_contract() {
    let job: WorkJob = serde_json::from_value(json!({
        "id": "0192f6aa-0000-7000-8000-000000000001", "type": "runner", "kind": "container",
        "attempt": 1, "lease": { "until": "2026-11-18T10:01:30Z" },
        "limits": { "seconds": 21600, "idleSeconds": 3600, "memoryBytes": 8_589_934_592_u64,
                    "diskBytes": 42_949_672_960_u64, "orphanSeconds": 600 },
        "project": "0192f000-0000-7000-8000-000000000002",
        "repository": { "forge": "github", "owner": "example", "name": "app" },
        "secrets": ["jit_config"], "parameters": {}
    }))
    .unwrap();
    let written = serde_json::to_value(&job).unwrap();
    assert_eq!(written["type"], "runner");
    assert_eq!(written["lease"]["until"], "2026-11-18T10:01:30Z");
    assert_eq!(written["limits"]["idleSeconds"], 3600);
    assert_eq!(written["secrets"], json!(["jit_config"]));
}

#[test]
fn k10_7_a_report_reads_with_and_without_its_log_and_its_result() {
    let heartbeat: WorkReport = serde_json::from_value(json!({
        "id": "0192f6aa-0000-7000-8000-000000000001", "attempt": 1, "state": "running",
        "log": { "from": 20480, "text": "a line\n" }, "result": null
    }))
    .unwrap();
    assert_eq!(heartbeat.state, WorkState::Running);
    assert_eq!(heartbeat.log.unwrap().from, 20480);
    let ended: WorkReport = serde_json::from_value(json!({
        "id": "0192f6aa-0000-7000-8000-000000000001", "attempt": 1, "state": "done",
        "result": { "outcome": "succeeded", "cause": null, "seconds": 412,
          "cleanup": { "isolation": "deleted", "checkout": "deleted", "secrets": "deleted", "keychain": "none" } }
    }))
    .unwrap();
    assert!(ended.state.is_final() && ended.log.is_none());
    assert_eq!(ended.result.unwrap().seconds, 412);
    assert!(
        serde_json::from_value::<WorkReport>(json!({
            "id": "0192f6aa-0000-7000-8000-000000000001", "attempt": 1, "state": "paused"
        }))
        .is_err()
    );
}

#[test]
fn k10_7_the_answer_to_a_report_names_a_stop_and_the_end_of_the_log() {
    let reported: Reported = serde_json::from_value(json!({
        "lease": { "until": "2026-11-18T10:03:00Z" }, "stop": { "reason": "runners_off" }, "logEnd": 4096
    }))
    .unwrap();
    assert_eq!(reported.stop.unwrap().reason, StopReason::RunnersOff);
    assert_eq!(reported.log_end, Some(4096));
    let quiet: Reported = serde_json::from_value(
        json!({ "lease": { "until": "2026-11-18T10:03:00Z" }, "stop": null }),
    )
    .unwrap();
    assert_eq!(serde_json::to_value(quiet).unwrap().get("logEnd"), None);
}

#[test]
fn k10_11_the_refusals_of_the_work_route_have_their_statuses() {
    for (code, status, text) in [
        (RefusalCode::NotHolder, 409, "NOT_HOLDER"),
        (RefusalCode::WorkOff, 403, "WORK_OFF"),
        (RefusalCode::SecretRefused, 409, "SECRET_REFUSED"),
    ] {
        assert_eq!((code.status(), code.as_str()), (status, text));
        assert_eq!(serde_json::to_value(code).unwrap(), text);
        assert!(!code.drops_batch());
    }
}
