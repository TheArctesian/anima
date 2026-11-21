//! The target of a route (SF-NEW-routetarget): never a session port, a port of Anima, an
//! address the hub must not touch, or a listener of the hub.

use std::net::{Ipv6Addr, SocketAddr};

use super::support::socket;
use crate::fleet::targets::AddressPolicy;
use crate::work::routes::target::{TargetRefusal, check_address, check_port};

/// An address in the range of the devices of the link.
const LINK_DEVICE: Ipv6Addr = Ipv6Addr::new(0xfd41, 0x4e49, 0x4d41, 0, 0, 0, 0, 7);

#[test]
fn a_session_port_is_never_the_port_of_an_application() {
    for port in [22, 5900, 3389] {
        assert_eq!(check_port(port), Err(TargetRefusal::SessionPort), "{port}");
    }
}

#[test]
fn a_port_of_anima_or_outside_the_range_is_refused() {
    assert_eq!(check_port(8443), Err(TargetRefusal::AnimaPort));
    assert_eq!(check_port(4822), Err(TargetRefusal::AnimaPort));
    assert_eq!(check_port(0), Err(TargetRefusal::Port));
    assert_eq!(check_port(65_536), Err(TargetRefusal::Port));
    assert_eq!(check_port(3000), Ok(3000));
}

#[test]
fn the_ports_of_the_engine_the_link_databases_and_metrics_are_refused() {
    for port in [2375, 2376, 10250] {
        assert_eq!(check_port(port), Err(TargetRefusal::EnginePort), "{port}");
    }
    for port in [41641, 3478] {
        assert_eq!(check_port(port), Err(TargetRefusal::AnimaPort), "{port}");
    }
    for port in [5432, 3306, 6379, 27017, 9090] {
        assert_eq!(check_port(port), Err(TargetRefusal::ServicePort), "{port}");
    }
}

#[test]
fn a_link_address_passes_only_with_the_port_of_an_application() {
    let policy = AddressPolicy::default();
    for port in [22, 2375, 8443, 5432] {
        let target = SocketAddr::new(LINK_DEVICE.into(), port);
        assert!(check_address(target, &policy, &[]).is_err(), "{port}");
    }
    let target = SocketAddr::new(LINK_DEVICE.into(), 3000);
    assert_eq!(check_address(target, &policy, &[]), Ok(target));
}

#[test]
fn loopback_link_local_and_metadata_addresses_are_refused() {
    let policy = AddressPolicy::default().allowing_loopback();
    for target in [
        "127.0.0.1:3000",
        "[::1]:3000",
        "169.254.169.254:80",
        "169.254.10.1:3000",
    ] {
        assert_eq!(
            check_address(socket(target), &policy, &[]),
            Err(TargetRefusal::Address),
            "{target}"
        );
    }
}

#[test]
fn a_listener_of_the_hub_is_refused_and_a_different_port_passes() {
    let policy = AddressPolicy::default();
    let hub = [socket("10.10.0.99:9443"), socket("0.0.0.0:443")];
    assert_eq!(
        check_address(socket("10.10.0.99:9443"), &policy, &hub),
        Err(TargetRefusal::Hub)
    );
    assert_eq!(
        check_address(socket("10.10.0.21:443"), &policy, &hub),
        Err(TargetRefusal::Hub)
    );
    assert_eq!(
        check_address(socket("10.10.0.21:3000"), &policy, &hub),
        Ok(socket("10.10.0.21:3000"))
    );
}
