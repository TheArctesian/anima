//! The match of a node to an Anima device and the node's name (VPN-008, VPN-011, BD-14).

use super::{device, device_id, json, peer_mut, picture_with, reparse};
use crate::vpn::Picture;

/// The idle fixture's mac with `dns_name` and `host_name`, matched against `devices`.
fn mac_matched(dns_name: &str, host_name: &str, devices: &[crate::devices::Device]) -> Picture {
    let mut raw = json("status-idle.json");
    let mac = peer_mut(&mut raw, "mac");
    mac["DNSName"] = dns_name.into();
    mac["HostName"] = host_name.into();
    picture_with(&reparse(&raw), devices, &[])
}

fn matched_device(shown: &Picture) -> Option<crate::ids::DeviceId> {
    shown
        .nodes
        .iter()
        .find(|node| node.address.octets()[3] == 2)
        .and_then(|node| node.device.as_ref().map(|device| device.0))
}

#[test]
fn vpn_011_the_address_outranks_the_name_and_the_name_outranks_the_label() {
    let devices = [
        device(8, "by-label", "mac"),
        device(9, "by-name", "mac.net.anima.ing"),
        device(10, "by-address", "100.64.0.2"),
    ];

    let all = mac_matched("mac.net.anima.ing.", "mac", &devices);
    let no_address = mac_matched("mac.net.anima.ing.", "mac", &devices[..2]);

    assert_eq!(matched_device(&all), Some(device_id(10)));
    assert_eq!(matched_device(&no_address), Some(device_id(9)));
}

#[test]
fn vpn_011_a_peer_with_no_dns_name_matches_no_device_without_an_address() {
    let devices = [device(9, "blank", ""), device(10, "dns", "mac.other")];

    let shown = mac_matched("", "mac", &devices);

    assert_eq!(matched_device(&shown), None);
}

#[test]
fn vpn_011_a_device_host_in_another_domain_is_not_the_label() {
    let devices = [device(9, "storage", "mac.other")];

    let shown = mac_matched("mac.net.anima.ing.", "mac", &devices);

    assert_eq!(matched_device(&shown), None);
}

#[test]
fn vpn_008_a_dns_name_of_one_dot_falls_back_to_the_host_name() {
    let shown = mac_matched(".", "laptop", &[]);

    assert!(shown.nodes.iter().any(|node| node.name == "laptop"));
}

#[test]
fn vpn_008_a_dns_label_is_cut_to_63_characters() {
    let dns = format!("{}.net.anima.ing.", "a".repeat(70));

    let shown = mac_matched(&dns, "mac", &[]);

    assert!(shown.nodes.iter().any(|node| node.name == "a".repeat(63)));
}
