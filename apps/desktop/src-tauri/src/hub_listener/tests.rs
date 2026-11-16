use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{
    HubListenerError, from_main,
    listeners::{self, Bound, Listener, Owner, Target},
    origin::{self, LoopbackOrigin},
    owner,
};

const V4: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const V6: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);

/// `/proc/net/tcp` of a Linux machine (little-endian): user 1000 listens on 127.0.0.1:8443,
/// user 1001 on 0.0.0.0:43123, user 1000 has a connection (state 01) on 9000.
const PROC_TCP: &str = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:20FB 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 31337 1 0000000000000000 100 0 0 10 0
   1: 00000000:A873 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1001        0 31338 1 0000000000000000 100 0 0 10 0
   2: 0100007F:2328 0100007F:C350 01 00000000:00000000 00:00000000 00000000  1000        0 31339 1 0000000000000000 20 4 30 10 -1
   3: garbage
";

/// `/proc/net/tcp6`: user 1000 listens on `[::1]:8443`.
const PROC_TCP6: &str = "\
  sl  local_address                         remote_address                        st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000000000000000000001000000:20FB 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 41337 1 0000000000000000 100 0 0 10 0
";

#[test]
fn only_the_main_web_view_may_ask() {
    assert_eq!(from_main("main"), Ok(()));
    assert_eq!(from_main("web-1"), Err(HubListenerError::Caller));
    let sent = serde_json::to_value(HubListenerError::BadOrigin).unwrap();
    assert_eq!(sent["code"], "bad-origin");
}

#[test]
fn only_the_three_loopback_origins_with_a_port_are_checked() {
    assert_eq!(
        origin::parse("http://127.0.0.1:8443"),
        Some(LoopbackOrigin {
            addresses: vec![V4],
            port: 8443
        })
    );
    assert_eq!(origin::parse("http://[::1]:1").unwrap().addresses, [V6]);
    assert_eq!(
        origin::parse("http://localhost:65535").unwrap().addresses,
        [V4, V6]
    );
    for bad in [
        "https://127.0.0.1:8443",
        "http://127.0.0.1",
        "http://127.0.0.1:",
        "http://127.0.0.1:0",
        "http://127.0.0.1:65536",
        "http://127.0.0.1:+80",
        "http://127.0.0.1:8443/",
        "http://127.0.0.2:8443",
        "http://localhost.evil.example:80",
        "http://user@127.0.0.1:8443",
        "http://hub.anima.ing:443",
        "",
    ] {
        assert_eq!(origin::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn the_proc_tables_give_listening_sockets_with_their_owners() {
    let found = listeners::proc_net_tcp(PROC_TCP, false);
    assert_eq!(
        found,
        [
            Listener {
                bound: Bound::At(V4),
                port: 8443,
                uid: Some(1000)
            },
            Listener {
                bound: Bound::At(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
                port: 43123,
                uid: Some(1001)
            },
        ]
    );
    assert_eq!(
        listeners::proc_net_tcp(PROC_TCP6, true),
        [Listener {
            bound: Bound::At(V6),
            port: 8443,
            uid: Some(1000)
        }]
    );
}

#[test]
fn a_port_is_own_only_when_every_listener_that_reaches_it_is_this_users() {
    let found = listeners::proc_net_tcp(PROC_TCP, false);

    assert!(listeners::only_own(&found, V4, 8443, 1000));
    assert!(!listeners::only_own(&found, V4, 8443, 1001));
    // Another user's listener on 0.0.0.0 reaches 127.0.0.1.
    assert!(!listeners::only_own(&found, V4, 43123, 1000));
    // An IPv4 listener does not reach ::1, and a connection is not a listener.
    assert!(!listeners::only_own(&found, V6, 8443, 1000));
    assert!(!listeners::only_own(&found, V4, 9000, 1000));

    let both = [
        found[0],
        Listener {
            bound: Bound::At(V4),
            port: 8443,
            uid: Some(1001),
        },
    ];
    assert!(!listeners::only_own(&both, V4, 8443, 1000));
}

#[test]
fn lsof_names_give_this_users_listeners() {
    let printed = "p501\nn127.0.0.1:8443\np502\nn*:43123\nn[::1]:8443\nnnonsense\n";

    let found = listeners::lsof_names(printed);

    assert_eq!(found.len(), 3);
    assert!(listeners::only_own(&found, V4, 8443, 501));
    assert!(listeners::only_own(&found, V6, 8443, 501));
    assert!(listeners::only_own(&found, V4, 43123, 501));
    assert!(!listeners::only_own(&found, V4, 9000, 501));
}

#[test]
fn a_listener_of_this_test_is_own_and_a_closed_port_has_nobody() {
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let origin = LoopbackOrigin {
        addresses: vec![V4],
        port,
    };

    assert_eq!(owner(&origin, true), Owner::Own);
    // The device link's port while the app's own child does not serve it.
    assert_eq!(owner(&origin, false), Owner::Other);
    drop(listener);
    assert_eq!(owner(&origin, true), Owner::Nobody);
}

#[test]
fn the_verdict_is_own_only_when_each_listened_address_is_own() {
    let target = |accepts, own| Target { accepts, own };

    assert_eq!(listeners::verdict(&[target(false, false)]), Owner::Nobody);
    assert_eq!(listeners::verdict(&[target(true, true)]), Owner::Own);
    assert_eq!(listeners::verdict(&[target(true, false)]), Owner::Other);
    // localhost: our hub on 127.0.0.1, nothing on ::1.
    assert_eq!(
        listeners::verdict(&[target(true, true), target(false, false)]),
        Owner::Own
    );
    // localhost: our hub on 127.0.0.1, another program on ::1.
    assert_eq!(
        listeners::verdict(&[target(true, true), target(true, false)]),
        Owner::Other
    );
    assert_eq!(serde_json::to_value(Owner::Nobody).unwrap(), "none");
    assert_eq!(serde_json::to_value(Owner::Own).unwrap(), "own");
    assert_eq!(serde_json::to_value(Owner::Other).unwrap(), "other");
}
