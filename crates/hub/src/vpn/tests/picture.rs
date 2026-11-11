//! The picture: roles, paths, keys, sites and the match to Anima devices (VPN-009 to VPN-011).

use chrono::Duration;

use super::{
    device, device_id, json, node, now, peer_mut, picture, picture_with, reparse, sites, status,
};
use crate::vpn::{
    KeyState, Picture, Role, RouteState, VpnPath,
    picture::{carry, router_devices},
};

/// The idle fixture with the router's path fields set.
fn router_with(active: bool, cur_addr: &str, peer_relay: &str, online: bool) -> Picture {
    let mut raw = json("status-idle.json");
    let router = peer_mut(&mut raw, "router-home");
    router["Active"] = active.into();
    router["CurAddr"] = cur_addr.into();
    router["PeerRelay"] = peer_relay.into();
    router["Online"] = online.into();
    picture(&reparse(&raw))
}

fn router_path(shown: &Picture) -> (VpnPath, Option<&str>) {
    let router = node(shown, "router-home");
    (router.path, router.relay_region.as_deref())
}

#[test]
fn vpn_009_offline_is_unknown_even_with_peer_relay_set() {
    let shown = router_with(true, "192.0.2.10:41641", "100.64.0.20:7777", false);

    assert_eq!(router_path(&shown), (VpnPath::Unknown, None));
}

#[test]
fn vpn_009_a_router_with_its_site_tag_is_a_router() {
    let shown = picture(&status("status-idle.json"));

    assert_eq!(node(&shown, "router-home").role, Role::Router);
    assert_eq!(shown.hub.role, Role::Hub);
    assert_eq!(node(&shown, "mac").role, Role::Client);
}

#[test]
fn vpn_009_roles_follow_the_first_matching_tag() {
    let shown = picture(&status("status-mixed.json"));

    assert_eq!(node(&shown, "printer").role, Role::Agent);
    let mut raw = json("status-idle.json");
    peer_mut(&mut raw, "mac")["Tags"] = serde_json::json!(["tag:other", "tag:hub"]);
    peer_mut(&mut raw, "router-home")["Tags"] = serde_json::json!(["tag:site-home"]);
    let changed = picture(&reparse(&raw));
    assert_eq!(node(&changed, "mac").role, Role::Hub);
    assert_eq!(node(&changed, "router-home").role, Role::Other);
}

#[test]
fn vpn_009_a_known_path_is_kept_for_three_unknown_reads_then_dropped() {
    let direct = picture(&status("status-active.json"));
    let idle = || picture(&status("status-idle.json"));

    let mut shown = direct;
    let mut paths = Vec::new();
    for _ in 0..4 {
        shown = carry(&shown, idle());
        paths.push(router_path(&shown).0);
    }

    assert_eq!(
        paths,
        [
            VpnPath::Direct,
            VpnPath::Direct,
            VpnPath::Direct,
            VpnPath::Unknown
        ]
    );
    let again = carry(&shown, picture(&status("status-active.json")));
    assert_eq!(node(&again, "router-home").unknown_reads, 0);
    assert_eq!(router_path(&carry(&again, idle())).0, VpnPath::Direct);
}

#[test]
fn vpn_009_an_offline_node_keeps_no_path() {
    let direct = picture(&status("status-active.json"));

    let offline = carry(&direct, router_with(false, "", "", false));

    assert_eq!(router_path(&offline), (VpnPath::Unknown, None));
}

#[test]
fn vpn_009_a_paused_ping_check_does_not_count() {
    // The reader leaves a paused check out of `ping_checked`; the picture reads only that set.
    let devices = [device(9, "router-home", "100.64.0.3")];

    let shown = picture_with(&status("status-idle.json"), &devices, &[]);

    assert!(!shown.sites[0].has_ping_check);
}

#[test]
fn vpn_009_active_with_cur_addr_is_direct() {
    assert_eq!(
        router_path(&picture(&status("status-active.json"))),
        (VpnPath::Direct, None)
    );
}

#[test]
fn vpn_009_active_without_cur_addr_is_relayed_with_region() {
    assert_eq!(
        router_path(&picture(&status("status-relayed.json"))),
        (VpnPath::Relayed, Some("sfo"))
    );
}

#[test]
fn vpn_009_peer_relay_wins() {
    let shown = router_with(true, "192.0.2.10:41641", "100.64.0.20:7777", true);

    assert_eq!(router_path(&shown), (VpnPath::PeerRelay, None));
    let mixed = picture(&status("status-mixed.json"));
    assert_eq!(node(&mixed, "router-office1").path, VpnPath::PeerRelay);
}

#[test]
fn vpn_009_idle_is_unknown_even_with_relay_set() {
    assert_eq!(
        router_path(&picture(&status("status-idle.json"))),
        (VpnPath::Unknown, None)
    );
}

#[test]
fn vpn_009_router_without_primary_routes_serves_no_site() {
    let shown = picture(&status("status-mixed.json"));

    let office2 = node(&shown, "router-office2");
    assert_eq!((office2.role, office2.in_anima), (Role::Router, false));
    assert_eq!(office2.routes.len(), 0);
    assert_eq!(shown.sites[0].state, RouteState::NoRouter);
    let office1 = &shown.sites[1];
    assert_eq!(office1.state, RouteState::Serving);
    assert_eq!(shown.nodes[office1.router.unwrap()].name, "router-office1");
}

#[test]
fn vpn_009_offline_router_gives_offline() {
    let shown = router_with(false, "", "", false);

    assert_eq!(shown.sites[0].state, RouteState::Offline);
    assert_eq!(shown.sites[1].state, RouteState::NoRouter);
    assert_eq!(shown.sites[1].router, None);
}

#[test]
fn vpn_009_ping_check_on_the_router_device_is_reported() {
    let devices = [device(9, "router-home", "100.64.0.3")];
    let parsed = status("status-idle.json");

    let shown = picture_with(&parsed, &devices, &[device_id(9)]);

    assert!(shown.sites[0].has_ping_check);
    assert!(!shown.sites[1].has_ping_check);
    assert_eq!(router_devices(&parsed, &sites(), &devices), [device_id(9)]);
}

#[test]
fn vpn_009_only_routers_that_serve_a_site_have_their_checks_read() {
    let devices = [
        device(9, "mac", "100.64.0.2"),
        device(10, "router-office2", "100.64.0.5"),
        device(11, "router-office1", "100.64.0.4"),
    ];

    let found = router_devices(&status("status-mixed.json"), &sites(), &devices);

    assert_eq!(found, [device_id(11)]);
}

#[test]
fn vpn_010_tagged_without_expiry_is_never() {
    let shown = picture(&status("status-idle.json"));

    assert_eq!(node(&shown, "router-home").key_state, KeyState::Never);
    assert_eq!(shown.hub.key_state, KeyState::Never);
}

#[test]
fn vpn_010_tagged_with_expiry_is_flagged() {
    let shown = picture(&status("status-mixed.json"));

    assert_eq!(
        node(&shown, "printer").key_state,
        KeyState::TaggedWithExpiry
    );
}

#[test]
fn vpn_010_expiry_in_20_days_is_expiring() {
    let shown = picture(&status("status-mixed.json"));

    let laptop = node(&shown, "laptop");
    assert_eq!(laptop.key_state, KeyState::Expiring);
    assert_eq!(laptop.key_expiry, Some(now() + Duration::days(20)));
    assert_eq!(node(&shown, "guest").key_state, KeyState::Valid);
}

#[test]
fn vpn_010_a_past_expiry_is_expired() {
    let mut raw = json("status-idle.json");
    peer_mut(&mut raw, "mac")["KeyExpiry"] = "2030-05-05T12:00:00Z".into();

    assert_eq!(
        node(&picture(&reparse(&raw)), "mac").key_state,
        KeyState::Expired
    );
}

#[test]
fn vpn_011_a_peer_matched_by_ip_is_in_anima() {
    let devices = [device(9, "laptop-device", "100.64.0.6")];

    let shown = picture_with(&status("status-mixed.json"), &devices, &[]);

    let laptop = node(&shown, "laptop");
    assert!(laptop.in_anima);
    assert_eq!(
        laptop.device,
        Some((device_id(9), "laptop-device".to_owned()))
    );
}

#[test]
fn vpn_011_a_device_whose_host_is_the_magicdns_name_is_matched() {
    let mut raw = json("status-mixed.json");
    peer_mut(&mut raw, "nas")["DNSName"] = "nas.net.anima.ing.".into();
    let devices = [device(9, "storage", "NAS.net.anima.ing")];

    let shown = picture_with(&reparse(&raw), &devices, &[]);

    assert_eq!(
        node(&shown, "nas").device.as_ref().map(|d| d.0),
        Some(device_id(9))
    );
}

#[test]
fn vpn_011_a_device_whose_host_is_the_first_label_is_matched() {
    let mut raw = json("status-mixed.json");
    peer_mut(&mut raw, "nas")["DNSName"] = "nas.net.anima.ing.".into();
    let devices = [device(9, "storage", "nas")];

    let shown = picture_with(&reparse(&raw), &devices, &[]);

    assert!(node(&shown, "nas").in_anima);
}

#[test]
fn vpn_011_a_device_with_the_same_label_in_another_domain_is_not_matched() {
    let mut raw = json("status-mixed.json");
    peer_mut(&mut raw, "nas")["DNSName"] = "nas.net.anima.ing.".into();
    let devices = [device(9, "storage", "nas.home.arpa")];

    let shown = picture_with(&reparse(&raw), &devices, &[]);

    assert!(!node(&shown, "nas").in_anima);
}

#[test]
fn vpn_011_a_peer_whose_host_name_copies_a_device_is_not_matched() {
    let devices = [device(9, "storage", "nas")];

    let shown = picture_with(&status("status-mixed.json"), &devices, &[]);

    let evil = node(&shown, "evil");
    assert_eq!((evil.in_anima, &evil.device), (false, &None));
}

#[test]
fn vpn_011_an_unmatched_client_is_not_in_anima() {
    let shown = picture(&status("status-mixed.json"));

    let guest = node(&shown, "guest");
    assert_eq!((guest.role, guest.in_anima), (Role::Client, false));
    assert!(shown.hub.in_anima);
}

#[test]
fn vpn_011_an_agent_node_matched_by_ip_is_in_anima() {
    let devices = [device(9, "printer", "100.64.0.7")];

    let shown = picture_with(&status("status-mixed.json"), &devices, &[]);

    let printer = node(&shown, "printer");
    assert_eq!((printer.role, printer.in_anima), (Role::Agent, true));
}

#[test]
fn vpn_009_nodes_are_in_name_order_and_sites_in_site_order() {
    let shown = picture(&status("status-mixed.json"));

    let names: Vec<&str> = shown.nodes.iter().map(|node| node.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    assert_eq!(
        shown.sites.iter().map(|site| site.site).collect::<Vec<_>>(),
        sites().iter().map(|site| site.0).collect::<Vec<_>>()
    );
}

/// The idle fixture with the router's routes and the mac's tags and routes replaced.
fn home_served_by(router: &[&str], mac_tags: &[&str], mac_routes: &[&str]) -> Picture {
    let mut raw = json("status-idle.json");
    peer_mut(&mut raw, "router-home")["PrimaryRoutes"] = serde_json::json!(router);
    let mac = peer_mut(&mut raw, "mac");
    mac["Tags"] = serde_json::json!(mac_tags);
    mac["PrimaryRoutes"] = serde_json::json!(mac_routes);
    picture(&reparse(&raw))
}

#[test]
fn vpn_009_a_router_with_a_wider_or_narrower_route_serves_no_site() {
    let devices = [device(9, "router-home", "100.64.0.3")];
    for route in ["10.10.0.0/16", "10.10.0.0/25"] {
        let mut raw = json("status-idle.json");
        peer_mut(&mut raw, "router-home")["PrimaryRoutes"] = serde_json::json!([route]);

        let shown = picture_with(&reparse(&raw), &devices, &[]);

        assert_eq!(shown.sites[0].state, RouteState::NoRouter, "{route}");
        assert_eq!(shown.sites[0].router, None, "{route}");
        assert!(!node(&shown, "router-home").in_anima, "{route}");
    }
}

#[test]
fn vpn_009_a_peer_that_is_not_a_router_serves_no_site() {
    for tags in [&[][..], &["tag:agent"][..]] {
        let shown = home_served_by(&[], tags, &["10.10.0.0/24"]);

        assert_eq!(shown.sites[0].state, RouteState::NoRouter, "{tags:?}");
        assert_eq!(shown.sites[0].router, None, "{tags:?}");
    }
}

#[test]
fn vpn_009_hub_outranks_router_when_a_node_has_both_tags() {
    let mut raw = json("status-idle.json");
    peer_mut(&mut raw, "mac")["Tags"] = serde_json::json!(["tag:router", "tag:hub"]);

    assert_eq!(node(&picture(&reparse(&raw)), "mac").role, Role::Hub);
}

#[test]
fn vpn_009_a_relayed_path_keeps_its_region_for_three_unknown_reads() {
    let idle = || picture(&status("status-idle.json"));

    let mut shown = picture(&status("status-relayed.json"));
    assert_eq!(router_path(&shown), (VpnPath::Relayed, Some("sfo")));
    for _ in 0..3 {
        shown = carry(&shown, idle());
        assert_eq!(router_path(&shown), (VpnPath::Relayed, Some("sfo")));
    }
    assert_eq!(
        router_path(&carry(&shown, idle())),
        (VpnPath::Unknown, None)
    );
}
