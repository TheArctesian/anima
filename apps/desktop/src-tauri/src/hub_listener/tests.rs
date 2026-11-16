use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{
    HubListenerError, from_main,
    listeners::{self, Listed, Listener, Owner, covers},
    origin, owner,
};
use crate::link::DeviceState;

const V4: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const V6: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
const ANY4: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const ANY6: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);

/// `/proc/net/tcp` of a Linux machine (little-endian): user 1000 listens on 127.0.0.1:8443,
/// user 1001 on 0.0.0.0:43123, user 1000 has a connection (state 01) on 9000.
const PROC_TCP: &str = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:20FB 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 31337 1 0000000000000000 100 0 0 10 0
   1: 00000000:A873 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1001        0 31338 1 0000000000000000 100 0 0 10 0
   2: 0100007F:2328 0100007F:C350 01 00000000:00000000 00:00000000 00000000  1000        0 31339 1 0000000000000000 20 4 30 10 -1
   3: garbage
";

/// `/proc/net/tcp6`: user 1000 on `[::1]:8443`, user 1001 on `[::]:5000` (dual stack) and
/// user 1002 on `[::ffff:127.0.0.1]:6000`.
const PROC_TCP6: &str = "\
  sl  local_address                         remote_address                        st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000000000000000000001000000:20FB 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 41337 1 0000000000000000 100 0 0 10 0
   1: 00000000000000000000000000000000:1388 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1001        0 41338 1 0000000000000000 100 0 0 10 0
   2: 0000000000000000FFFF00000100007F:1770 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1002        0 41339 1 0000000000000000 100 0 0 10 0
";

fn listener(address: IpAddr, port: u16, uid: Option<u32>) -> Listener {
    Listener { address, port, uid }
}

#[test]
fn only_the_main_web_view_may_ask() {
    assert_eq!(from_main("main"), Ok(()));
    assert_eq!(from_main("web-1"), Err(HubListenerError::Caller));
    let sent = serde_json::to_value(HubListenerError::BadOrigin).unwrap();
    assert_eq!(sent["code"], "bad-origin");
}

#[test]
fn only_http_127_0_0_1_with_a_port_is_checked() {
    assert_eq!(origin::port("http://127.0.0.1:8443"), Some(8443));
    assert_eq!(origin::port("http://127.0.0.1:65535"), Some(65535));
    for bad in [
        "http://localhost:8443",
        "http://[::1]:8443",
        "https://127.0.0.1:8443",
        "http://127.0.0.1",
        "http://127.0.0.1:",
        "http://127.0.0.1:0",
        "http://127.0.0.1:65536",
        "http://127.0.0.1:+80",
        "http://127.0.0.1:8443/",
        "http://127.0.0.2:8443",
        "http://user@127.0.0.1:8443",
        "",
    ] {
        assert_eq!(origin::port(bad), None, "{bad:?}");
    }
}

#[test]
fn a_wildcard_reaches_its_own_family_and_on_linux_a_dual_stack_one_reaches_ipv4() {
    let mapped = IpAddr::V6(Ipv4Addr::LOCALHOST.to_ipv6_mapped());
    let table = [
        (V4, V4, Listed::ThisUser, true),
        (ANY4, V4, Listed::ThisUser, true),
        (ANY4, V6, Listed::ThisUser, false),
        (ANY4, V6, Listed::EveryUser, false),
        (ANY6, V6, Listed::ThisUser, true),
        (ANY6, V4, Listed::ThisUser, false),
        (ANY6, V4, Listed::EveryUser, true),
        (mapped, V4, Listed::ThisUser, true),
        (V6, V4, Listed::EveryUser, false),
        (V4, V6, Listed::EveryUser, false),
    ];
    for (bound, target, listed, reaches) in table {
        assert_eq!(
            covers(bound, target, listed),
            reaches,
            "{bound} {target} {listed:?}"
        );
    }
}

#[test]
fn the_proc_tables_give_listening_sockets_with_their_owners() {
    assert_eq!(
        listeners::proc_net_tcp(PROC_TCP, false),
        [
            listener(V4, 8443, Some(1000)),
            listener(ANY4, 43123, Some(1001))
        ]
    );
    assert_eq!(
        listeners::proc_net_tcp(PROC_TCP6, true),
        [
            listener(V6, 8443, Some(1000)),
            listener(ANY6, 5000, Some(1001)),
            // An IPv4-mapped address is the IPv4 address.
            listener(V4, 6000, Some(1002)),
        ]
    );
}

#[test]
fn on_linux_a_port_is_own_only_when_every_listener_that_reaches_it_is_this_users() {
    let mut found = listeners::proc_net_tcp(PROC_TCP, false);
    found.extend(listeners::proc_net_tcp(PROC_TCP6, true));
    let own = |port, uid| listeners::only_own(&found, Listed::EveryUser, (V4, port), uid);

    assert!(own(8443, 1000));
    assert!(!own(8443, 1001));
    // Another user's listener on 0.0.0.0, on dual-stack `::`, or on a mapped address.
    assert!(!own(43123, 1000));
    assert!(!own(5000, 1000));
    assert!(own(6000, 1002));
    assert!(!own(6000, 1000));
    // A connection is not a listener.
    assert!(!own(9000, 1000));

    let both = [
        listener(V4, 8443, Some(1000)),
        listener(V4, 8443, Some(1001)),
    ];
    assert!(!listeners::only_own(
        &both,
        Listed::EveryUser,
        (V4, 8443),
        1000
    ));
}

#[test]
fn lsof_names_give_this_users_listeners_with_the_family_of_a_wildcard() {
    let printed = "p501\nf7\ntIPv4\nn127.0.0.1:8443\np502\nf8\ntIPv6\nn*:43123\nf9\ntIPv6\nn[::1]:8443\nf10\nn*:7000\nnnonsense\n";

    let found = listeners::lsof_names(printed);

    assert_eq!(
        found,
        [
            listener(V4, 8443, None),
            listener(ANY6, 43123, None),
            listener(V6, 8443, None)
        ]
    );
    assert!(listeners::only_own(
        &found,
        Listed::ThisUser,
        (V4, 8443),
        501
    ));
    // This user's IPv6 wildcard does not vouch for 127.0.0.1.
    assert!(!listeners::only_own(
        &found,
        Listed::ThisUser,
        (V4, 43123),
        501
    ));
}

#[test]
fn an_ipv4_wildcard_of_this_user_does_not_vouch_for_ipv6() {
    let found = listeners::lsof_names("p501\nf7\ntIPv4\nn*:8443\n");

    assert_eq!(found, [listener(ANY4, 8443, None)]);
    assert!(listeners::only_own(
        &found,
        Listed::ThisUser,
        (V4, 8443),
        501
    ));
    assert!(!listeners::only_own(
        &found,
        Listed::ThisUser,
        (V6, 8443),
        501
    ));
}

#[test]
fn the_device_link_port_is_permitted_only_while_its_own_child_serves_it() {
    let starting = DeviceState::default().after(crate::link::Event::Starting { port: 43123 });
    let up = starting.after(crate::link::Event::Ready { port: 43123 });
    let down = up.after(crate::link::Event::Ended);

    assert!(listeners::link_permits(&DeviceState::default(), 43123));
    assert!(listeners::link_permits(&down, 8443));
    assert!(listeners::link_permits(&up, 43123));
    assert!(!listeners::link_permits(&starting, 43123));
    assert!(!listeners::link_permits(&down, 43123));
}

#[test]
fn a_listener_of_this_test_is_own_and_a_closed_port_has_nobody() {
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();

    assert_eq!(owner(port, true), Owner::Own);
    // The device link's port while the app's own child does not serve it.
    assert_eq!(owner(port, false), Owner::Other);
    drop(listener);
    assert_eq!(owner(port, true), Owner::Nobody);
}

#[test]
fn the_verdict_is_own_only_for_this_users_listener() {
    assert_eq!(listeners::verdict(false, false), Owner::Nobody);
    assert_eq!(listeners::verdict(true, true), Owner::Own);
    assert_eq!(listeners::verdict(false, true), Owner::Own);
    assert_eq!(listeners::verdict(true, false), Owner::Other);
    assert_eq!(serde_json::to_value(Owner::Nobody).unwrap(), "none");
    assert_eq!(serde_json::to_value(Owner::Own).unwrap(), "own");
    assert_eq!(serde_json::to_value(Owner::Other).unwrap(), "other");
}
