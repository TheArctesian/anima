//! `anima device trust` against the scripted hub: a host key or certificate is accepted only by
//! the fingerprint the operator gives (SEC-006, SEC-007).

use serde_json::json;

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-device-trust-tests";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

#[tokio::test]
async fn sec_006_accept_host_key_sends_the_fingerprint_and_prints_what_was_accepted() {
    let hub = FakeHub::start().await;
    hub.answer(
        "HostKeyAccept",
        data(json!({ "acceptHostKey": { "algorithm": "ssh-ed25519",
            "fingerprintSha256": "SHA256:abc", "firstSeenAt": "2026-11-13T09:00:00+00:00" } })),
    );
    let run = signed_in()
        .run(&[
            "device",
            "trust",
            "accept-host-key",
            DEVICE,
            "SHA256:abc",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("SHA256:abc") && run.stdout.contains("ssh-ed25519"));
    let seen = hub.seen();
    assert_eq!(
        seen[0].variables,
        json!({ "deviceId": DEVICE, "fingerprintSha256": "SHA256:abc" })
    );
}

#[tokio::test]
async fn sec_006_nothing_is_accepted_without_a_fingerprint() {
    let hub = FakeHub::start().await;
    for args in [
        &["device", "trust", "accept-host-key", DEVICE][..],
        &["device", "trust", "accept-certificate", DEVICE, ""][..],
    ] {
        let run = signed_in()
            .run(&[args, &["--hub", &hub.origin]].concat())
            .await;
        assert_eq!(run.code, 2, "{run:?}");
    }
    assert!(hub.seen().is_empty(), "no request without a fingerprint");
}

#[tokio::test]
async fn sec_007_a_fingerprint_the_hub_did_not_see_is_conflict_exit_6() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CertificateAccept",
        refusal("CONFLICT", "That is not the certificate the hub saw."),
    );
    let run = signed_in()
        .run(&[
            "device",
            "trust",
            "accept-certificate",
            DEVICE,
            "SHA256:other",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 6, "{run:?}");
    assert!(run.stderr.contains("not the certificate the hub saw"));
}
