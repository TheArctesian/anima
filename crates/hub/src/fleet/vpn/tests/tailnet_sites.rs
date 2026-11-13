//! Sites on the tailnet's own range are reached directly and are not router sites; nor are sites
//! over the hub's own container networks (VP-13).

use super::{now, site, status};
use crate::fleet::vpn::Picture;

fn picture_of_sites(sites: &[(crate::ids::SiteId, ipnet::IpNet)]) -> Picture {
    use crate::fleet::vpn::{
        BrokerState,
        picture::{Inputs, build},
    };
    build(&Inputs {
        status: &status("status-idle.json"),
        sites,
        devices: &[],
        ping_checked: &std::collections::HashSet::new(),
        now: now(),
        broker_state: BrokerState::NotSet,
        broker: None,
    })
    .unwrap()
}

#[test]
fn vpn_009_a_site_on_the_tailnet_range_is_not_a_router_site() {
    let shown = picture_of_sites(&[
        site(1, "10.10.0.0/24"),
        site(3, "100.64.0.0/10"),
        site(4, "100.64.5.0/24"),
        site(5, "fd7a:115c:a1e0:ab::/64"),
    ]);

    let ids: Vec<_> = shown.sites.iter().map(|path| path.site).collect();
    assert_eq!(ids, [uuid::Uuid::from_u128(1).into()]);
}

#[test]
fn vpn_009_a_subnet_that_contains_the_tailnet_range_is_not_a_router_site() {
    use crate::fleet::vpn::picture::on_tailnet;
    for subnet in ["0.0.0.0/0", "100.0.0.0/8", "::/0"] {
        assert!(on_tailnet(subnet.parse().unwrap()), "{subnet}");
    }
    for subnet in [
        "10.10.0.0/24",
        "100.128.0.0/10",
        "192.168.1.0/24",
        "fd00::/16",
    ] {
        assert!(!on_tailnet(subnet.parse().unwrap()), "{subnet}");
    }
}

#[test]
fn vp_13_a_site_over_the_hub_networks_is_not_a_router_site() {
    let shown = picture_of_sites(&[
        site(1, "10.10.0.0/24"),
        site(2, "172.30.0.0/24"),
        site(3, "172.30.0.10/32"),
        site(4, "172.16.0.0/12"),
        site(5, "172.31.0.0/24"),
    ]);

    let ids: Vec<_> = shown.sites.iter().map(|path| path.site).collect();
    assert_eq!(
        ids,
        [
            uuid::Uuid::from_u128(1).into(),
            uuid::Uuid::from_u128(5).into()
        ]
    );
}

#[test]
fn vp_13_a_subnet_overlaps_the_hub_networks_when_either_contains_the_other() {
    use crate::fleet::vpn::picture::on_hub_networks;
    for subnet in ["172.30.0.0/16", "172.30.4.0/24", "172.0.0.0/8", "0.0.0.0/0"] {
        assert!(on_hub_networks(subnet.parse().unwrap()), "{subnet}");
    }
    for subnet in [
        "172.31.0.0/16",
        "172.17.0.0/16",
        "10.10.0.0/24",
        "fd00::/8",
        "::/0",
    ] {
        assert!(!on_hub_networks(subnet.parse().unwrap()), "{subnet}");
    }
}
