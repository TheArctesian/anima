//! `anima sealed-record` against the scripted hub (K-E5 Section 3; VLT-001): the commands move
//! opaque bytes and never print them unless asked.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const OWNER: &str = "passport:0190a000-0000-7000-8000-00000000aaaa";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-sealed-record-tests")
}

fn record(sealed: &[u8]) -> Value {
    json!({ "id": "0190a000-0000-7000-9500-000000000001", "owner": OWNER,
            "extension": "documents", "sealed": STANDARD.encode(sealed), "sealedIndex": null,
            "keyId": STANDARD.encode([7_u8; 32]), "createdAt": "2026-11-20T10:00:00+00:00",
            "updatedAt": "2026-11-20T10:00:00+00:00" })
}

#[tokio::test]
async fn vlt_001_sealed_record_store_sends_the_bytes_of_the_file_as_base64() {
    let hub = FakeHub::start().await;
    let sealed = [0_u8, 1, 2, 250, 251, 252, 253];
    hub.answer(
        "SealedRecordStore",
        data(json!({ "storeSealedRecord": record(&sealed) })),
    );
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("sealed.bin");
    std::fs::write(&file, sealed).unwrap();
    let key_id = STANDARD.encode([7_u8; 32]);
    let run = signed_in()
        .run(&[
            "sealed-record",
            "store",
            OWNER,
            "--extension",
            "documents",
            "--from-file",
            file.to_str().unwrap(),
            "--key-id",
            &key_id,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("Sealed bytes") && run.stdout.contains('7'),
        "{}",
        run.stdout
    );
    let sent = &hub.seen()[0].variables["input"];
    assert_eq!(sent["sealed"], STANDARD.encode(sealed));
    assert_eq!(sent["owner"], OWNER);
    assert!(sent["sealedIndex"].is_null());
}

#[tokio::test]
async fn vlt_001_sealed_record_show_prints_the_size_and_writes_the_bytes_to_a_file() {
    let hub = FakeHub::start().await;
    let sealed = b"sealed bytes of a client";
    hub.answer(
        "SealedRecordShow",
        data(json!({ "sealedRecord": record(sealed) })),
    );
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("out.bin");
    let run = signed_in()
        .run(&[
            "sealed-record",
            "show",
            OWNER,
            "--to-file",
            file.to_str().unwrap(),
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("24"), "{}", run.stdout);
    assert!(
        !run.stdout.contains(&STANDARD.encode(sealed)),
        "{}",
        run.stdout
    );
    assert_eq!(std::fs::read(&file).unwrap(), sealed);
}

#[tokio::test]
async fn vlt_001_sealed_record_show_of_a_record_with_none_is_not_found() {
    let hub = FakeHub::start().await;
    hub.answer("SealedRecordShow", data(json!({ "sealedRecord": null })));
    let run = signed_in()
        .run(&["sealed-record", "show", OWNER, "--hub", &hub.origin])
        .await;
    assert_ne!(run.code, 0, "{run:?}");
    assert!(
        run.stderr.contains("has no sealed record"),
        "{}",
        run.stderr
    );
}

#[tokio::test]
async fn vlt_001_sealed_record_remove_with_yes_deletes_it() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SealedRecordRemove",
        data(json!({ "deleteSealedRecord": true })),
    );
    let run = signed_in()
        .run(&[
            "sealed-record",
            "remove",
            OWNER,
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(hub.seen()[0].variables["owner"], OWNER);
}
