//! `anima vpn` against the scripted hub: the picture, and the pre-auth keys that are shown once
//! and stay out of everything else (VPN-001, VPN-003, CMD-014, LOG-007).

use std::os::unix::fs::PermissionsExt as _;

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-vpn-tests";
const KEY: &str = "pre-auth-key-9f3a1c7e5b2d4a60";
const SITE: &str = "0190a000-0000-7000-8001-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

fn node(name: &str, role: &str) -> Value {
    json!({
        "name": name, "address": "100.64.0.2", "os": "linux", "role": role, "tags": [],
        "routes": [], "waitingRoutes": ["10.10.0.0/24"], "online": true,
        "lastSeen": null, "keyExpiry": "2027-02-01T00:00:00+00:00", "keyState": "VALID",
        "path": "DIRECT", "relayRegion": null, "inAnima": true,
    })
}

fn picture() -> Value {
    data(json!({ "vpn": {
        "readAt": "2026-11-12T10:00:00+00:00", "fresh": true, "broker": "READY",
        "hub": node("anima-hub", "HUB"),
        "sites": [{
            "site": { "id": SITE, "name": "home" }, "router": null, "state": "NO_ROUTER",
            "path": "UNKNOWN", "relayRegion": null, "hasPingCheck": false,
            "waitingRouter": { "name": "home-router" },
        }],
        "nodes": [node("laptop", "CLIENT")],
    }}))
}

fn key_answer(field: &str) -> Value {
    data(json!({ field: {
        "key": KEY, "expiresAt": "2026-11-12T11:00:00+00:00",
        "steps": ["install tailscale", "tailscale up --auth-key=file:/path"],
    }}))
}

#[tokio::test]
async fn vpn_status_prints_the_hub_each_site_and_each_node() {
    let hub = FakeHub::start().await;
    hub.answer("VpnStatus", picture());
    let run = signed_in()
        .run(&["vpn", "status", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    for text in [
        "READY",
        "Hub",
        "Site home",
        "NO_ROUTER",
        "home-router",
        "Node laptop",
    ] {
        assert!(run.stdout.contains(text), "{text} missing: {}", run.stdout);
    }
    let run = signed_in()
        .run(&["vpn", "status", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["sites"][0]["site"]["name"], "home");
}

#[tokio::test]
async fn vpn_status_without_a_picture_says_so_and_json_is_null() {
    let hub = FakeHub::start().await;
    hub.answer("VpnStatus", data(json!({ "vpn": null })));
    let run = signed_in()
        .run(&["vpn", "status", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("no picture yet"), "{}", run.stdout);
    let run = signed_in()
        .run(&["vpn", "status", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json(), Value::Null);
}

#[tokio::test]
async fn vpn_001_router_key_resolves_the_site_name_and_prints_the_key_once() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SiteNames",
        data(json!({ "sites": [{ "id": SITE, "name": "home" }] })),
    )
    .answer("VpnRouterKey", key_answer("createRouterKey"));
    let run = signed_in()
        .run(&["vpn", "router-key", "--site", "home", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout, format!("{KEY}\n"), "only the key on stdout");
    assert!(run.stderr.contains("shown one time"), "{}", run.stderr);
    assert!(run.stderr.contains("install tailscale"), "{}", run.stderr);
    assert!(!run.stderr.contains(KEY), "{}", run.stderr);
    let seen = hub.seen();
    let make = seen
        .iter()
        .find(|s| s.operation == "VpnRouterKey")
        .expect("a call");
    assert_eq!(make.variables["siteId"], SITE);
}

#[tokio::test]
async fn vpn_003_invite_json_holds_the_key_and_the_steps() {
    let hub = FakeHub::start().await;
    hub.answer("VpnInvite", key_answer("createVpnInvite"));
    let run = signed_in()
        .run(&["vpn", "invite", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let json = run.json();
    assert_eq!(json["key"], KEY);
    assert_eq!(json["steps"][0], "install tailscale");
    assert_eq!(run.stderr, "");
}

#[tokio::test]
async fn cmd_014_a_key_written_with_to_file_is_0600_and_never_on_stdout_or_stderr() {
    let hub = FakeHub::start().await;
    hub.answer("VpnInvite", key_answer("createVpnInvite"));
    let dir = tempfile::tempdir().expect("a folder");
    let path = dir.path().join("vpn.key");
    let path_text = path.to_str().expect("a path");
    let run = signed_in()
        .run(&[
            "vpn",
            "invite",
            "--to-file",
            path_text,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    run.assert_never_shows(KEY);
    assert_eq!(run.stdout, "", "nothing secret on stdout");
    assert!(run.stderr.contains(path_text), "{}", run.stderr);
    assert_eq!(std::fs::read_to_string(&path).expect("the key file"), KEY);
    let mode = std::fs::metadata(&path)
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);

    let json_path = dir.path().join("json.key");
    let run = signed_in()
        .run(&[
            "vpn",
            "invite",
            "--json",
            "--to-file",
            json_path.to_str().expect("a path"),
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    run.assert_never_shows(KEY);
    assert_eq!(run.json()["expiresAt"], "2026-11-12T11:00:00+00:00");
}

#[tokio::test]
async fn cmd_014_a_failed_key_request_shows_no_key_and_exits_5_when_forbidden() {
    let hub = FakeHub::start().await;
    hub.answer(
        "VpnInvite",
        refusal("FORBIDDEN", "An API key cannot make VPN keys."),
    );
    let run = signed_in()
        .run(&["vpn", "invite", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert_eq!(run.stdout, "");
}

#[tokio::test]
async fn vpn_register_and_renew_print_the_node() {
    let hub = FakeHub::start().await;
    let node = json!({ "name": "laptop", "address": "100.64.0.9", "keyExpiry": "2027-03-01T00:00:00+00:00" });
    hub.answer("VpnRegister", data(json!({ "registerVpnNode": node })))
        .answer("VpnRenew", data(json!({ "renewVpnKey": node })));
    let run = signed_in()
        .run(&["vpn", "register", "abc-123", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("100.64.0.9"), "{}", run.stdout);
    let run = signed_in()
        .run(&["vpn", "renew", "laptop", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["keyExpiry"], "2027-03-01T00:00:00+00:00");
    let seen = hub.seen();
    assert_eq!(seen[0].variables["authId"], "abc-123");
    assert_eq!(seen[1].variables["name"], "laptop");
}

#[tokio::test]
async fn vpn_approve_route_names_the_site_by_its_id() {
    let hub = FakeHub::start().await;
    hub.answer("VpnApproveRoute", data(json!({ "approveSiteRoute": true })));
    let run = signed_in()
        .run(&["vpn", "approve-route", "--site", SITE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("yes"), "{}", run.stdout);
    assert_eq!(hub.seen()[0].variables["siteId"], SITE);

    let hub = FakeHub::start().await;
    hub.answer("SiteNames", data(json!({ "sites": [] })));
    let run = signed_in()
        .run(&[
            "vpn",
            "approve-route",
            "--site",
            "nowhere",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}

#[tokio::test]
async fn cmd_014_s5_to_file_refuses_an_existing_file_before_asking_the_hub_unless_forced() {
    let hub = FakeHub::start().await;
    hub.answer("VpnInvite", key_answer("createVpnInvite"));
    let dir = tempfile::tempdir().expect("a folder");
    let path = dir.path().join("vpn.key");
    std::fs::write(&path, "precious").expect("a file");
    let path_text = path.to_str().expect("a path");
    let args = [
        "vpn",
        "invite",
        "--to-file",
        path_text,
        "--hub",
        &hub.origin,
    ];
    let run = signed_in().run(&args).await;
    assert_eq!(run.code, 6, "{run:?}");
    assert!(run.stderr.contains("--force"), "{}", run.stderr);
    assert!(
        hub.seen().is_empty(),
        "no key was made for a file that cannot be written"
    );
    assert_eq!(std::fs::read_to_string(&path).expect("kept"), "precious");

    let forced = [
        "vpn",
        "invite",
        "--to-file",
        path_text,
        "--force",
        "--hub",
        &hub.origin,
    ];
    let run = signed_in().run(&forced).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(std::fs::read_to_string(&path).expect("replaced"), KEY);
}

#[tokio::test]
async fn cmd_014_s5_a_key_with_control_characters_is_cleaned_on_the_terminal() {
    let hub = FakeHub::start().await;
    hub.answer(
        "VpnInvite",
        data(json!({ "createVpnInvite": {
            "key": "abc\u{1b}[2Jdef\u{202E}", "expiresAt": "2026-11-12T11:00:00+00:00",
            "steps": ["one"],
        }})),
    );
    let run = signed_in()
        .run(&["vpn", "invite", "--hub", &hub.origin])
        .await;
    assert_eq!(run.stdout, "abc [2Jdef\n");
}

#[tokio::test]
async fn cmd_020_s5_the_steps_note_keeps_one_step_to_a_line() {
    let hub = FakeHub::start().await;
    hub.answer("VpnInvite", key_answer("createVpnInvite"));
    let run = signed_in()
        .run(&["vpn", "invite", "--hub", &hub.origin])
        .await;
    assert!(
        run.stderr
            .contains("in order:\n  1. install tailscale\n  2. tailscale up"),
        "{}",
        run.stderr
    );
}
