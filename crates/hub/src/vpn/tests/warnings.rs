//! The warnings a picture calls for (VPN-010, VPN-011).

use chrono::Duration;

use super::{device, json, now, peer_mut, picture, picture_with, reparse, status};
use crate::{
    events::Severity,
    vpn::warnings::{Kind, Warning, due},
};

/// The warnings for the idle fixture, with the mac's key expiring `days` after the clock and
/// both peers known to Anima.
fn mac_warnings(expiry: Duration) -> Vec<Warning> {
    let mut raw = json("status-idle.json");
    let at = (now() + expiry).to_rfc3339();
    peer_mut(&mut raw, "mac")["KeyExpiry"] = at.into();
    let devices = [device(9, "mac", "100.64.0.2")];
    due(&picture_with(&reparse(&raw), &devices, &[]), now())
}

#[test]
fn vpn_010_kind_is_the_smallest_threshold_not_below_the_days_left() {
    let cases = [
        (31, None),
        (30, Some(Kind::Expiry30)),
        (15, Some(Kind::Expiry30)),
        (14, Some(Kind::Expiry14)),
        (8, Some(Kind::Expiry14)),
        (7, Some(Kind::Expiry7)),
        (2, Some(Kind::Expiry7)),
        (1, Some(Kind::Expiry1)),
        (0, Some(Kind::Expiry1)),
        (-3, Some(Kind::Expiry1)),
    ];
    for (days, kind) in cases {
        let kinds: Vec<Kind> = mac_warnings(Duration::days(days))
            .iter()
            .map(|warning| warning.kind)
            .collect();
        assert_eq!(kinds, kind.into_iter().collect::<Vec<_>>(), "{days} days");
    }
}

#[test]
fn vpn_010_a_part_day_counts_as_a_whole_day() {
    let warnings = mac_warnings(Duration::days(7) + Duration::hours(1));

    assert_eq!(warnings[0].kind, Kind::Expiry14);
    assert_eq!(
        warnings[0].message,
        "VPN key of mac expires in 8 days, on 2030-05-12"
    );
}

#[test]
fn vpn_010_an_expiring_key_names_the_node_the_days_and_the_date() {
    let warnings = mac_warnings(Duration::days(20));

    let warning = &warnings[0];
    assert_eq!(
        warning.message,
        "VPN key of mac expires in 20 days, on 2030-05-25"
    );
    assert_eq!(warning.severity, Severity::Warning);
    assert_eq!(warning.key_expiry, Some(now() + Duration::days(20)));
    assert_eq!(warning.device.as_ref().map(|d| d.1.as_str()), Some("mac"));
    assert!(warning.key.starts_with("nodekey:"));
}

#[test]
fn vpn_010_the_last_day_and_an_expired_key_are_errors() {
    assert_eq!(
        mac_warnings(Duration::hours(20))[0].severity,
        Severity::Error
    );
    let expired = &mac_warnings(-Duration::days(3))[0];
    assert_eq!(expired.severity, Severity::Error);
    assert_eq!(expired.message, "VPN key of mac expired on 2030-05-02");
}

#[test]
fn vpn_010_a_tagged_key_with_an_expiry_is_warned_once_for_the_node() {
    let warnings = due(&picture(&status("status-mixed.json")), now());

    let tagged: Vec<&Warning> = warnings
        .iter()
        .filter(|warning| warning.kind == Kind::TaggedExpiry)
        .collect();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].key_expiry, None);
    assert_eq!(
        tagged[0].message,
        "VPN node printer has a tag and a key expiry; tagged keys must not expire (VPN-003)"
    );
}

#[test]
fn vpn_011_a_node_outside_anima_is_warned_with_its_address() {
    let warnings = due(&picture(&status("status-mixed.json")), now());

    let messages: Vec<&str> = warnings
        .iter()
        .filter(|warning| warning.kind == Kind::NotInAnima)
        .map(|warning| warning.message.as_str())
        .collect();
    assert!(messages.contains(&"VPN node guest (100.64.0.8) is not in Anima"));
    assert!(messages.contains(&"VPN router router-office2 (100.64.0.5) serves no Anima site"));
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("router-office1"))
    );
    assert!(!messages.iter().any(|message| message.contains("hub")));
}

#[test]
fn vpn_010_the_hub_key_is_warned_too() {
    let mut raw = json("status-idle.json");
    raw["Self"]["Tags"] = serde_json::Value::Null;
    raw["Self"]["KeyExpiry"] = (now() + Duration::days(5)).to_rfc3339().into();

    let warnings = due(&picture(&reparse(&raw)), now());

    assert!(
        warnings
            .iter()
            .any(|w| w.kind == Kind::Expiry7 && w.message.contains(" hub "))
    );
}

#[test]
fn vpn_010_a_node_whose_key_the_marks_cannot_hold_gets_no_warning() {
    let mut raw = json("status-idle.json");
    peer_mut(&mut raw, "mac")["PublicKey"] = "PublicKey:0000".into();

    let warnings = due(&picture(&reparse(&raw)), now());

    assert!(
        warnings
            .iter()
            .all(|warning| warning.key.starts_with("nodekey:"))
    );
    assert!(
        !warnings
            .iter()
            .any(|warning| warning.message.contains("mac"))
    );
}

#[test]
fn vpn_010_a_node_key_over_the_mark_limit_gets_no_warning_and_80_characters_do() {
    for (length, warned) in [(100, false), (81, false), (80, true)] {
        let mut raw = json("status-idle.json");
        let key = format!("nodekey:{}", "0".repeat(length - "nodekey:".len()));
        peer_mut(&mut raw, "mac")["PublicKey"] = key.clone().into();

        let warnings = due(&picture(&reparse(&raw)), now());

        assert_eq!(warnings.iter().any(|w| w.key == key), warned, "{length}");
    }
}

#[test]
fn vpn_008_debug_output_leaves_the_node_key_out() {
    let shown = picture(&status("status-idle.json"));
    let warnings = due(&shown, now());

    let text = format!("{shown:?} {warnings:?}");

    assert_ne!(warnings, []);
    assert!(!text.contains("nodekey"), "{text}");
}

#[test]
fn vpn_010_a_warning_kind_has_its_stored_text() {
    let stored = [
        Kind::Expiry30,
        Kind::Expiry14,
        Kind::Expiry7,
        Kind::Expiry1,
        Kind::TaggedExpiry,
        Kind::NotInAnima,
    ]
    .map(Kind::as_str);

    assert_eq!(
        stored,
        [
            "expiry_30",
            "expiry_14",
            "expiry_7",
            "expiry_1",
            "tagged_expiry",
            "not_in_anima"
        ]
    );
}
