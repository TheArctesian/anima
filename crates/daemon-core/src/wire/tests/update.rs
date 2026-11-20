//! The update on the wire: the `update` member of a batch, the instruction of `Accepted`, the
//! bodies of the two update routes and `NO_RELEASE` (K8 Section 8, R-139).

use super::{assert_accepted, assert_malformed, checked};
use crate::wire::{
    BatchUpdate, MAX_RECORD_BYTES, MAX_SIGNATURE_BYTES, PushAccepted, Refusal, RefusalCode,
    ReleaseRecord, Switch, TryCause, TryResult, UpdateInstruction, UpdateRequest, UpdateTry,
};

/// The K8 Section 8.1 example.
const EXAMPLE: &str = r#"{
  "target": "linux-x86_64",
  "switch": "on",
  "previous": "0.2.3",
  "busy": false,
  "last": { "version": "0.3.0", "result": "rolled_back", "cause": "no_check", "tries": 1 }
}"#;

/// A change of the example member.
type Change = fn(&mut BatchUpdate);

fn example() -> BatchUpdate {
    serde_json::from_str(EXAMPLE).unwrap()
}

fn tried(result: TryResult, cause: Option<TryCause>, tries: u8) -> UpdateTry {
    UpdateTry {
        version: "0.3.0".to_owned(),
        result,
        cause,
        tries,
    }
}

#[test]
fn r_139_a_batch_hands_over_what_the_daemon_says_of_its_update() {
    let update = example();
    assert_eq!(update.switch, Switch::On);
    assert_eq!(
        update.last,
        Some(tried(TryResult::RolledBack, Some(TryCause::NoCheck), 1))
    );
    let accepted = checked(|batch| batch.update = Some(update.clone())).unwrap();
    assert_eq!(accepted.update, Some(update));
    assert_eq!(
        checked(|_| {}).unwrap().update,
        None,
        "absent: no update function (ROL-044)"
    );
}

#[test]
fn r_139_a_cause_the_hub_does_not_know_reads_as_failed() {
    let text = EXAMPLE.replace("no_check", "cosmic_ray");
    let update: BatchUpdate = serde_json::from_str(&text).unwrap();
    assert_eq!(update.last.unwrap().cause, Some(TryCause::Failed));
    assert_accepted("an unknown cause of a refused try", |batch| {
        let mut update = example();
        update.last = Some(tried(TryResult::Refused, Some(TryCause::Failed), 2));
        batch.update = Some(update);
    });
}

#[test]
fn r_139_a_target_of_its_form_that_the_hub_does_not_know_is_accepted() {
    assert_accepted("an unknown target", |batch| {
        let mut update = example();
        update.target = "freebsd-riscv64".to_owned();
        batch.update = Some(update);
    });
}

#[test]
fn r_139_an_update_member_that_breaks_its_rules_is_malformed() {
    let installed = |cause| Some(tried(TryResult::Installed, cause, 1));
    let cases: [(&str, Change); 9] = [
        ("an empty target", |u| u.target = String::new()),
        ("a target with a space", |u| u.target = "linux x86".into()),
        ("a target over 64 bytes", |u| u.target = "a".repeat(65)),
        ("a previous that is not a version", |u| {
            u.previous = Some("v1".into());
        }),
        ("a last version that is not a version", |u| {
            u.last.as_mut().unwrap().version = String::new();
        }),
        ("tries 0", |u| u.last.as_mut().unwrap().tries = 0),
        ("tries 3", |u| u.last.as_mut().unwrap().tries = 3),
        ("a rolled back try with no cause", |u| {
            u.last.as_mut().unwrap().cause = None;
        }),
        ("a rollback cause on a refused try", |u| {
            *u.last.as_mut().unwrap() = tried(TryResult::Refused, Some(TryCause::NoCheck), 1);
        }),
    ];
    for (case, change) in cases {
        assert_malformed(case, |batch| {
            let mut update = example();
            change(&mut update);
            batch.update = Some(update);
        });
    }
    for cause in [Some(TryCause::Asked), Some(TryCause::Failed)] {
        assert_malformed("an installed try with a cause", |batch| {
            let mut update = example();
            update.last = installed(cause);
            batch.update = Some(update);
        });
    }
    assert_malformed("a refusal cause on a rolled back try", |batch| {
        let mut update = example();
        update.last = Some(tried(TryResult::RolledBack, Some(TryCause::NoSpace), 1));
        batch.update = Some(update);
    });
}

#[test]
fn r_139_accepted_carries_the_version_to_have() {
    let told: PushAccepted =
        serde_json::from_str(r#"{ "update": { "version": "0.3.0", "again": false } }"#).unwrap();
    assert_eq!(
        told.update,
        Some(UpdateInstruction {
            version: "0.3.0".to_owned(),
            again: false
        })
    );
    assert_eq!(told.keys, None);
    let quiet = serde_json::to_string(&PushAccepted::default()).unwrap();
    assert_eq!(
        quiet, "{}",
        "no member is written that the hub does not tell"
    );
}

#[test]
fn r_139_the_update_routes_name_a_version_and_a_target_only() {
    let request: UpdateRequest =
        serde_json::from_str(r#"{ "version": "0.3.0", "target": "linux-x86_64" }"#).unwrap();
    assert_eq!(request.target, "linux-x86_64");
    let refusal: Refusal = serde_json::from_str(r#"{ "error": "NO_RELEASE" }"#).unwrap();
    assert_eq!(refusal.error, RefusalCode::NoRelease);
    assert_eq!(RefusalCode::NoRelease.status(), 404);
    assert_eq!(RefusalCode::NoRelease.as_str(), "NO_RELEASE");
}

#[test]
fn r_139_a_release_record_crosses_as_base64_within_its_limits() {
    let bytes = [0, 255, b'{', b'\n'];
    let body = ReleaseRecord::new(&bytes, "untrusted comment: x\nRW...").unwrap();
    let read: ReleaseRecord = serde_json::from_str(&serde_json::to_string(&body).unwrap()).unwrap();
    let (record, signature) = read.parts().unwrap();
    assert_eq!(record, bytes);
    assert_eq!(signature, "untrusted comment: x\nRW...");

    assert!(ReleaseRecord::new(&vec![0; MAX_RECORD_BYTES + 1], "s").is_none());
    assert!(ReleaseRecord::new(&[0], &"s".repeat(MAX_SIGNATURE_BYTES + 1)).is_none());
    let not_base64 = ReleaseRecord {
        record: "%%".to_owned(),
        signature: String::new(),
    };
    assert_eq!(not_base64.parts(), None);
}
