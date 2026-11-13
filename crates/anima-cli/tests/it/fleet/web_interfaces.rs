//! `anima web` against the scripted hub (CMD-020, CMD-021, CMD-024).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-web-tests";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const WEB: &str = "0190a000-0000-7000-8009-000000000001";
const CHECK: &str = "0190a000-0000-7000-8003-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

fn web(pinned: Option<&str>) -> Value {
    json!({
        "id": WEB, "deviceId": DEVICE, "label": "Admin", "url": "https://nas.lan:8443/",
        "openIn": "ANIMA_TAB", "pinnedFingerprintSha256": pinned,
        "pinnedAt": pinned.map(|_| "2026-11-13T09:00:00+00:00"),
    })
}

fn variables(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .unwrap_or_else(|| panic!("no {operation} request"))
        .variables
}

#[tokio::test]
async fn cmd_020_web_list_prints_the_interfaces_and_names_the_suggestions() {
    let hub = FakeHub::start().await;
    hub.answer(
        "WebList",
        data(json!({ "device": { "id": DEVICE, "name": "nas",
            "webInterfaces": [web(Some("SHA256:pin"))],
            "webInterfaceSuggestions": [{ "checkId": CHECK, "url": "http://nas.lan/", "label": "nas.lan" }],
        }})),
    );
    let run = signed_in()
        .run(&["web", "list", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run
        .stdout
        .lines()
        .find(|line| line.starts_with(WEB))
        .expect("a row");
    assert!(
        row.contains("Admin") && row.contains("ANIMA_TAB") && row.contains("SHA256:pin"),
        "{row}"
    );
    assert!(
        run.stderr
            .contains(&format!("anima web suggestion accept {CHECK}")),
        "{}",
        run.stderr
    );
    assert_eq!(variables(&hub, "WebList")["id"], DEVICE);

    let run = signed_in()
        .run(&["web", "list", DEVICE, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["webInterfaceSuggestions"][0]["checkId"], CHECK);
    assert_eq!(run.stderr, "");
}

#[tokio::test]
async fn cmd_020_web_create_and_update_send_the_input_flags() {
    let hub = FakeHub::start().await;
    hub.answer(
        "WebCreate",
        data(json!({ "createWebInterface": web(None) })),
    )
    .answer(
        "WebUpdate",
        data(json!({ "updateWebInterface": web(None) })),
    );
    let run = signed_in()
        .run(&[
            "web",
            "create",
            DEVICE,
            "--url",
            "https://nas.lan:8443/",
            "--label",
            "Admin",
            "--open-in",
            "system-browser",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        variables(&hub, "WebCreate"),
        json!({ "deviceId": DEVICE, "input": {
            "url": "https://nas.lan:8443/", "label": "Admin", "openIn": "SYSTEM_BROWSER" } })
    );
    assert!(run.stdout.contains(WEB));

    let run = signed_in()
        .run(&[
            "web",
            "update",
            WEB,
            "--url",
            "https://nas.lan/",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        variables(&hub, "WebUpdate"),
        json!({ "id": WEB, "input": { "url": "https://nas.lan/" } })
    );
}

#[tokio::test]
async fn cmd_024_web_delete_needs_yes() {
    let hub = FakeHub::start().await;
    hub.answer("WebDelete", data(json!({ "deleteWebInterface": WEB })));
    let run = signed_in()
        .run(&["web", "delete", WEB, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(hub.seen().is_empty());

    let run = signed_in()
        .run(&["web", "delete", WEB, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains(WEB), "{}", run.stdout);
}

#[tokio::test]
async fn cmd_020_a_suggestion_is_accepted_or_dismissed_by_its_check() {
    let hub = FakeHub::start().await;
    hub.answer(
        "WebSuggestionAccept",
        data(json!({ "acceptWebInterfaceSuggestion": web(None) })),
    )
    .answer(
        "WebSuggestionDismiss",
        data(json!({ "dismissWebInterfaceSuggestion": CHECK })),
    );
    let run = signed_in()
        .run(&["web", "suggestion", "accept", CHECK, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(variables(&hub, "WebSuggestionAccept")["checkId"], CHECK);
    assert!(run.stdout.contains("https://nas.lan:8443/"));

    let run = signed_in()
        .run(&["web", "suggestion", "dismiss", CHECK, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains(CHECK), "{}", run.stdout);
}

#[tokio::test]
async fn wb_008_accept_certificate_pins_the_given_fingerprint_and_a_mismatch_is_exit_6() {
    let hub = FakeHub::start().await;
    hub.answer(
        "WebCertificateAccept",
        data(json!({ "acceptWebCertificate": web(Some("SHA256:pin")) })),
    );
    let run = signed_in()
        .run(&[
            "web",
            "accept-certificate",
            WEB,
            "SHA256:pin",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("SHA256:pin"), "{}", run.stdout);
    assert_eq!(
        variables(&hub, "WebCertificateAccept"),
        json!({ "id": WEB, "fingerprintSha256": "SHA256:pin" })
    );

    let refusing = FakeHub::start().await;
    refusing.answer(
        "WebCertificateAccept",
        refusal("CONFLICT", "The certificate changed."),
    );
    let run = signed_in()
        .run(&[
            "web",
            "accept-certificate",
            WEB,
            "SHA256:old",
            "--hub",
            &refusing.origin,
        ])
        .await;
    assert_eq!(run.code, 6, "{run:?}");

    let run = signed_in()
        .run(&["web", "accept-certificate", WEB, "--hub", &refusing.origin])
        .await;
    assert_eq!(run.code, 2, "no fingerprint, no pin: {run:?}");
}
