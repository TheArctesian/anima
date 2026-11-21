//! The target of a route (SF-NEW-routetarget): never a session port, a port of Anima, an
//! address the hub must not touch, or a listener of the hub.

use super::support::socket;
use crate::fleet::targets::AddressPolicy;
use crate::work::routes::target::{TargetRefusal, check_address, check_port};

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
    let hub = [socket("10.10.0.99:8443"), socket("0.0.0.0:443")];
    assert_eq!(
        check_address(socket("10.10.0.99:8443"), &policy, &hub),
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
