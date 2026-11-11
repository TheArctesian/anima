//! The status parser and the cleaning of node text (VPN-008).

use super::{fixture, json, node, peer_mut, picture, reparse, status};
use crate::vpn::{Error, peers::parse};

#[test]
fn vpn_008_status_with_no_peers_parses() {
    let parsed = status("status-no-peers.json");

    assert!(parsed.peer.is_none());
    assert_eq!(picture(&parsed).nodes.len(), 0);
}

#[test]
fn vpn_008_zero_last_seen_is_none() {
    let parsed = status("status-idle.json");

    assert_eq!(parsed.own.last_seen, None);
    assert!(
        parsed
            .peer
            .unwrap()
            .values()
            .all(|peer| peer.last_seen.is_none())
    );
}

#[test]
fn vpn_008_a_set_last_seen_is_kept() {
    let mut raw = json("status-idle.json");
    let mac = peer_mut(&mut raw, "mac");
    mac["Online"] = false.into();
    mac["LastSeen"] = "2030-05-05T11:00:00Z".into();

    let shown = picture(&reparse(&raw));

    assert_eq!(
        node(&shown, "mac").last_seen.unwrap().to_rfc3339(),
        "2030-05-05T11:00:00+00:00"
    );
}

#[test]
fn vpn_008_unknown_fields_are_ignored() {
    let mut raw = json("status-idle.json");
    raw["NewTopLevel"] = serde_json::json!({ "x": 1 });
    peer_mut(&mut raw, "mac")["SomethingNew"] = serde_json::json!([1, 2]);

    assert_eq!(picture(&reparse(&raw)).nodes.len(), 2);
}

#[test]
fn vpn_008_host_name_is_cut_to_63_printable_characters() {
    let shown = picture(&status("status-mixed.json"));

    let long = shown
        .nodes
        .iter()
        .find(|node| node.name.starts_with("xxx"))
        .unwrap();
    assert_eq!(long.name.chars().count(), 63);
    assert!(crate::text::is_plain(&long.name));
}

#[test]
fn vpn_008_the_long_name_keeps_its_text_after_the_control_characters() {
    let mut raw = json("status-mixed.json");
    let long = raw["Peer"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .find(|peer| peer["DNSName"] == "")
        .unwrap();
    long["HostName"] = format!("{}\u{7}\u{202e}tail", "x".repeat(55)).into();

    let shown = picture(&reparse(&raw));

    assert!(
        shown
            .nodes
            .iter()
            .any(|node| node.name == format!("{}tail", "x".repeat(55)))
    );
}

#[test]
fn vpn_008_a_name_of_only_control_characters_becomes_a_question_mark() {
    let mut raw = json("status-idle.json");
    let mac = peer_mut(&mut raw, "mac");
    mac["DNSName"] = "".into();
    mac["HostName"] = "\u{7}\u{202e}\u{200b}".into();
    mac["OS"] = "\u{1b}".into();

    let shown = picture(&reparse(&raw));

    let cleaned = shown
        .nodes
        .iter()
        .find(|node| node.address.octets()[3] == 2)
        .unwrap();
    assert_eq!((cleaned.name.as_str(), cleaned.os.as_str()), ("?", "?"));
}

#[test]
fn vpn_008_null_tailscale_ips_parse_as_empty() {
    let parsed = status("status-mixed.json");

    let peers = parsed.peer.as_ref().unwrap();
    assert!(peers.values().any(|peer| peer.tailscale_ips.is_empty()));
    assert!(
        picture(&parsed)
            .nodes
            .iter()
            .all(|node| node.name != "no-address")
    );
}

#[test]
fn vpn_008_the_name_is_the_first_label_of_the_dns_name() {
    let mut raw = json("status-idle.json");
    peer_mut(&mut raw, "mac")["HostName"] = "Operator's MacBook".into();

    let shown = picture(&reparse(&raw));

    assert_eq!(node(&shown, "mac").address.to_string(), "100.64.0.2");
}

#[test]
fn vpn_008_a_reply_of_another_shape_is_unreadable() {
    for body in [
        "",
        "[]",
        "{\"Self\": null}",
        &fixture("status-idle.json").replace("\"Online\"", "\"Offline\""),
        &fixture("status-idle.json").replace("\"100.64.0.1\"", "\"not an address\""),
    ] {
        assert!(
            matches!(parse(body.as_bytes()), Err(Error::Unreadable)),
            "{body:.40}"
        );
    }
}

#[test]
fn vpn_008_any_bytes_give_a_status_or_unreadable() {
    let base = fixture("status-mixed.json").into_bytes();
    // Cut and flip the reply at many places: the parser never panics.
    for at in (0..base.len()).step_by(97) {
        let _ = parse(&base[..at]);
        let mut flipped = base.clone();
        flipped[at] ^= 0x55;
        let _ = parse(&flipped);
    }
}

#[test]
fn vpn_009_the_hub_reads_the_real_tailscale_captures() {
    // The A4 contract harness wrote these from Headscale 0.29.7 and Tailscale (vpn-build.md Section 3).
    let router = |name: &str| node(&picture(&status(name)), "router-home").path;
    assert_eq!(
        router("captured-status-idle.json"),
        crate::vpn::VpnPath::Unknown
    );
    assert_eq!(
        router("captured-status-active.json"),
        crate::vpn::VpnPath::Direct
    );
    assert_eq!(
        router("captured-status-after-60s.json"),
        crate::vpn::VpnPath::Unknown
    );
}

#[test]
fn vpn_008_an_unreadable_peer_is_skipped_and_the_rest_are_kept() {
    let mut raw = json("status-idle.json");
    let peers = raw["Peer"].as_object_mut().unwrap();
    peers.insert(
        "nodekey:odd".to_owned(),
        serde_json::json!({ "Online": "yes" }),
    );
    peers.insert("nodekey:none".to_owned(), serde_json::json!(7));

    let shown = picture(&reparse(&raw));

    let names: Vec<&str> = shown.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["mac", "router-home"]);
}

#[test]
fn vpn_008_an_unreadable_self_or_peer_list_is_still_unreadable() {
    let mut own = json("status-idle.json");
    own["Self"] = serde_json::json!({ "Online": 1 });
    let mut list = json("status-idle.json");
    list["Peer"] = serde_json::json!([1, 2]);

    for raw in [own, list] {
        assert!(matches!(
            parse(raw.to_string().as_bytes()),
            Err(Error::Unreadable)
        ));
    }
}
