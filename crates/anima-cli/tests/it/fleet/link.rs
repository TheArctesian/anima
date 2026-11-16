//! `anima link devices`, `invite` and `revoke` against the scripted hub: the device list, the
//! invite shown once and nowhere else, and the confirmed revocation (FRN-009, LNK-003, LNK-004,
//! CMD-014, LOG-007).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-link-tests";
const INVITE: &str = "anima-invite.1.ABCDEFGHIJKLMNOPQRSTUVWX";
const KEY: &str = "nodekey:5c1a00aa";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

pub fn device() -> Value {
    json!({
        "key": KEY, "fingerprint": "1a2b3c4d", "name": "laptop", "address": "fd41:4e49:4d41::7",
        "path": "RELAYED", "relay": "derp-ams", "expiresAt": "2030-08-03T12:00:00+00:00",
        "streams": 2,
    })
}

fn invite_answer() -> Value {
    data(json!({ "createLinkInvite": {
        "invite": INVITE, "expiresAt": "2030-05-06T12:00:00+00:00",
    }}))
}

#[tokio::test]
async fn lnk_004_link_devices_prints_each_device_with_its_path_and_key() {
    let hub = FakeHub::start().await;
    hub.answer("LinkDevices", data(json!({ "linkDevices": [device()] })));
    let run = signed_in()
        .run(&["link", "devices", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = (run.stdout.lines())
        .find(|line| line.starts_with("laptop"))
        .expect("a row");
    for text in ["1a2b3c4d", "fd41:4e49:4d41::7", "RELAYED (derp-ams)", KEY] {
        assert!(row.contains(text), "{text} missing: {row}");
    }
    let run = signed_in()
        .run(&["link", "devices", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json(), json!([device()]));
}

#[tokio::test]
async fn lnk_003_link_invite_prints_only_the_invite_on_stdout_and_the_expiry_on_stderr() {
    let hub = FakeHub::start().await;
    hub.answer("LinkInvite", invite_answer());
    let run = signed_in()
        .run(&["link", "invite", "phone", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        run.stdout,
        format!("{INVITE}\n"),
        "only the invite on stdout"
    );
    assert!(run.stderr.contains("2030-05-06T12:00:00"), "{}", run.stderr);
    assert!(run.stderr.contains("shown one time"), "{}", run.stderr);
    assert!(!run.stderr.contains(INVITE), "{}", run.stderr);
    assert_eq!(hub.seen()[0].variables, json!({ "name": "phone" }));

    let run = signed_in()
        .run(&["link", "invite", "phone", "--json", "--hub", &hub.origin])
        .await;
    let json = run.json();
    assert_eq!(json["invite"], INVITE);
    assert_eq!(json["expiresAt"], "2030-05-06T12:00:00+00:00");
    assert_eq!(json["name"], "phone");
}

#[tokio::test]
async fn cmd_014_an_invite_written_with_to_file_never_reaches_stdout_or_stderr() {
    let hub = FakeHub::start().await;
    hub.answer("LinkInvite", invite_answer());
    let dir = tempfile::tempdir().expect("a folder");
    let path = dir.path().join("invite.txt");
    let path_text = path.to_str().expect("a path");
    let run = signed_in()
        .run(&[
            "link",
            "invite",
            "phone",
            "--to-file",
            path_text,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    run.assert_never_shows(INVITE);
    assert_eq!(std::fs::read_to_string(&path).expect("the file"), INVITE);
}

#[tokio::test]
async fn d_23_an_api_key_is_refused_an_invite_with_exit_5_and_no_invite() {
    let hub = FakeHub::start().await;
    hub.answer(
        "LinkInvite",
        refusal(
            "FORBIDDEN",
            "This token cannot do that. Sign in to the hub to do it.",
        ),
    );
    let run = Anima::new()
        .with(
            "ANIMA_TOKEN",
            "anima_Ab12Cd34Ef56Gh78Ij90Kl12Mn34Op56Qr78St90Uv1",
        )
        .run(&["link", "invite", "phone", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert_eq!(run.stdout, "");
}

#[tokio::test]
async fn lnk_004_link_revoke_asks_first_then_sends_the_key_and_prints_the_streams_closed() {
    let hub = FakeHub::start().await;
    hub.answer("LinkRevoke", data(json!({ "revokeLinkDevice": 3 })));
    let run = signed_in()
        .run(&["link", "revoke", KEY, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "no one to ask: {run:?}");
    assert!(run.stderr.contains("--yes"), "{}", run.stderr);
    assert_eq!(hub.seen().len(), 0, "nothing sent without a confirmation");

    let run = signed_in()
        .run(&["link", "revoke", KEY, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("Streams closed"), "{}", run.stdout);
    assert!(run.stdout.contains('3'), "{}", run.stdout);
    assert_eq!(hub.seen()[0].variables, json!({ "key": KEY }));
}

#[tokio::test]
async fn frn_009_a_hub_with_no_link_is_unavailable() {
    let hub = FakeHub::start().await;
    hub.answer(
        "LinkDevices",
        refusal(
            "UNAVAILABLE",
            "This hub has no device link (ANIMA_LINK_CONTROL is not set).",
        ),
    );
    let run = signed_in()
        .run(&["link", "devices", "--hub", &hub.origin])
        .await;
    assert_ne!(run.code, 0, "{run:?}");
    assert!(run.stderr.contains("no device link"), "{}", run.stderr);
}
