//! The broker's reply lines and their merge into the picture (R-055, VP-7, BD-9, BD-16,
//! LOG-007). The socket itself is tested in `tests/it/vpn/broker.rs`.

use std::collections::HashSet;

use proptest::prelude::*;
use serde_json::{Value, json};

use super::{now, sites, status};
use crate::vpn::{
    BrokerState, Error, KeyState, Picture, Refusal, Role, VpnPath,
    broker::{BrokerView, NewKey, parse_reply},
    picture::{Inputs, build},
    reader::broker_answer,
};

const KEY: &str = "REDACTED-PREAUTH-KEY";

fn ok(value: &Value) -> Vec<u8> {
    json!({ "ok": true, "value": value })
        .to_string()
        .into_bytes()
}

/// A broker node as `broker/nodes.ts` writes it.
fn broker_node(id: u64, name: &str, (ip, tags): (&str, &[&str]), routes: [&[&str]; 2]) -> Value {
    let [approved, available] = routes;
    json!({ "id": id, "name": name, "user": "tagged-devices", "tags": tags, "ips": [ip],
            "online": true, "expiry": null, "approved": approved, "available": available })
}

/// `router-home` serving home with a second route waiting, and `router-office` waiting for
/// the office1 subnet; the policy names both sites.
fn view() -> BrokerView {
    let reply = json!({
        "nodes": [
            broker_node(3, "router-home", ("100.64.0.3", &["tag:router", "tag:site-home"]),
                [&["10.10.0.0/24"], &["10.10.0.0/24", "10.30.0.0/24"]]),
            broker_node(9, "router-office", ("100.64.0.9", &["tag:router", "tag:site-office1"]),
                [&[], &["10.20.0.0/24"]]),
        ],
        "sites": [{ "name": "home", "subnet": "10.10.0.0/24" },
                  { "name": "office1", "subnet": "10.20.0.0/24" }],
    });
    parse_reply(&ok(&reply)).unwrap()
}

fn merged(view: &BrokerView) -> Picture {
    let ping_checked = HashSet::new();
    build(&Inputs {
        status: &status("status-idle.json"),
        sites: &sites(),
        devices: &[],
        ping_checked: &ping_checked,
        now: now(),
        broker_state: BrokerState::Ready,
        broker: Some(view),
    })
    .unwrap()
}

#[test]
fn log_007_a_key_reply_debug_prints_no_key() {
    let line = ok(&json!({ "key": KEY, "expiresAt": "2030-05-05T13:00:00.000Z", "keyId": 4 }));

    let key: NewKey = parse_reply(&line).unwrap();

    assert_eq!(key.key.expose(), KEY);
    assert_eq!(key.expires_at, now() + chrono::Duration::hours(1));
    let printed = format!("{key:?}");
    assert!(!printed.contains(KEY), "{printed}");
    assert!(printed.contains("[pre-auth key]"), "{printed}");
}

#[test]
fn log_007_an_unreadable_key_reply_error_holds_no_reply_text() {
    // serde quotes a value it cannot read; the hub's error must not.
    let line = ok(&json!({ "key": KEY, "expiresAt": KEY }));

    let error = parse_reply::<NewKey>(&line).unwrap_err();

    assert!(matches!(error, Error::BrokerUnreadable));
    assert!(!format!("{error} {error:?}").contains(KEY));
}

#[test]
fn vp_7_a_refusal_keeps_its_word_and_cleaned_detail() {
    let line = br#"{"ok":false,"refusal":"undone","detail":"the new node was deleted: the node had a tag\u0007"}"#;

    let error = parse_reply::<BrokerView>(line).unwrap_err();

    let Error::BrokerRefused { refusal, detail } = error else {
        panic!("not a refusal: {error:?}");
    };
    assert_eq!(refusal, Refusal::Undone);
    assert_eq!(detail, "the new node was deleted: the node had a tag");
}

#[test]
fn vp_7_a_reply_of_another_shape_is_unreadable() {
    for line in [
        r#"{"ok":true}"#,
        r#"{"ok":true,"value":{"nodes":[],"sites":[]},"refusal":"undone"}"#,
        r#"{"ok":false,"refusal":"no-such-word","detail":""}"#,
        r#"{"ok":true,"value":{"nodes":[],"sites":[]},"extra":1}"#,
        r#"{"ok":true,"value":{"nodes":[{"id":-1}],"sites":[]}}"#,
        "[]",
        "",
    ] {
        let error = parse_reply::<BrokerView>(line.as_bytes()).unwrap_err();
        assert!(matches!(error, Error::BrokerUnreadable), "{line}");
    }
}

proptest! {
    /// Any bytes: no panic, and an error never repeats the line (LOG-007).
    #[test]
    fn log_007_any_reply_line_parses_or_fails_without_its_text(line in proptest::collection::vec(any::<u8>(), 0..512)) {
        if let Err(error) = parse_reply::<BrokerView>(&line) {
            let text = format!("{error} {error:?}");
            let shown = String::from_utf8_lossy(&line);
            prop_assert!(shown.len() < 8 || !text.contains(shown.as_ref()));
        }
    }

    /// A key reply of any key text with a bad expiry never shows the key in the error.
    #[test]
    fn log_007_a_bad_key_reply_never_shows_the_key(key in "[A-Za-z0-9_-]{8,64}") {
        let line = ok(&json!({ "key": key, "expiresAt": key }));
        let error = parse_reply::<NewKey>(&line).unwrap_err();
        let text = format!("{error} {error:?}");
        prop_assert!(!text.contains(&key));
    }
}

#[test]
fn vp_7_nodes_reply_is_merged_as_waiting_routes() {
    let shown = merged(&view());

    let router = super::node(&shown, "router-home");
    assert_eq!(router.waiting_routes, ["10.30.0.0/24".parse().unwrap()]);
    assert_eq!(
        super::node(&shown, "mac").waiting_routes,
        Vec::<ipnet::IpNet>::new()
    );
    assert_eq!(shown.broker, BrokerState::Ready);
    assert!(shown.roster.is_some());
}

#[test]
fn vp_7_a_router_waiting_for_its_site_subnet_is_the_waiting_router() {
    let shown = merged(&view());

    assert!(
        shown.sites[0].waiting_router.is_none(),
        "home's route is approved"
    );
    let waiting = shown.sites[1].waiting_router.as_ref().unwrap();
    assert_eq!(waiting.name, "router-office");
    assert_eq!(waiting.address.to_string(), "100.64.0.9");
    assert_eq!(waiting.role, Role::Router);
    assert_eq!(waiting.path, VpnPath::Unknown);
    assert_eq!(waiting.key_state, KeyState::Never);
    assert_eq!(waiting.waiting_routes, ["10.20.0.0/24".parse().unwrap()]);
    assert!(waiting.key.is_empty() && !waiting.in_anima);
}

#[test]
fn vp_7_a_router_without_the_site_tag_or_a_policy_site_does_not_wait_for_it() {
    let mut other_tag = view();
    other_tag.nodes[1].tags = vec!["tag:router".to_owned(), "tag:site-home".to_owned()];
    assert!(merged(&other_tag).sites[1].waiting_router.is_none());

    let mut no_policy = view();
    no_policy.sites.truncate(1);
    assert!(merged(&no_policy).sites[1].waiting_router.is_none());

    let mut odd_name = view();
    odd_name.sites[1].0 = "office 1; rm -rf /".to_owned();
    assert!(merged(&odd_name).sites[1].waiting_router.is_none());
}

#[test]
fn bd_15_a_busy_broker_read_keeps_the_last_state_and_node_list() {
    let last = merged(&view());
    let busy = Some(Err(Error::BrokerBusy));

    let (state, roster) = broker_answer(&busy, Some(&last));
    assert_eq!(state, BrokerState::Ready);
    assert_eq!(roster, Some(&view()));

    assert_eq!(broker_answer(&busy, None), (BrokerState::Failed, None));
    let failed = Some(Err(Error::BrokerTimedOut));
    assert_eq!(
        broker_answer(&failed, Some(&last)),
        (BrokerState::Failed, None)
    );
}
