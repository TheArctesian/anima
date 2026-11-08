//! The address policy `Live` hands the SSH probe (R-030, P2-004, WP-23 H-4): the deny list
//! applies to the loopback test `Live` in either order, and the probe never connects to a
//! denied address.

use std::time::Duration;

use tokio::net::TcpListener;

use crate::{
    sessions::Live,
    ssh::{SshError, Target, probe::read_host_key},
    targets::AddressPolicy,
};

const SHORT: Duration = Duration::from_millis(300);

#[test]
fn r_030_the_loopback_test_live_still_obeys_the_deny_list_in_either_order() {
    let deny = || AddressPolicy::new(vec!["127.0.0.2/32".parse().unwrap()]);
    let before = Live::default()
        .denying(deny())
        .reaching_loopback_for_tests();
    let after = Live::default()
        .reaching_loopback_for_tests()
        .denying(deny());
    for live in [before, after] {
        assert!(!live.policy().permits("127.0.0.2".parse().unwrap()));
        assert!(live.policy().permits("127.0.0.1".parse().unwrap()));
    }
    let strict = Live::default().denying(deny());
    assert!(!strict.policy().permits("127.0.0.1".parse().unwrap()));
}

/// An SSH session's target obeys the hub's deny list (WP-23 H-4, P2-004): the policy comes
/// from the `Live` that `main.rs` builds with [`Live::denying`], and the probe never
/// connects to a denied address, however the host names it.
#[tokio::test]
async fn r_030_the_probe_never_connects_to_a_denied_address() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let live = Live::default()
        .reaching_loopback_for_tests()
        .denying(AddressPolicy::new(vec!["127.0.0.1/32".parse().unwrap()]));
    for host in ["127.0.0.1", "localhost", "::ffff:127.0.0.1"] {
        let target = Target {
            host,
            port,
            policy: live.policy(),
        };
        let read = read_host_key(target, SHORT).await;
        assert_eq!(read, Err(SshError::RefusedTarget), "{host}");
    }
    let accepted = tokio::time::timeout(Duration::from_millis(50), listener.accept()).await;
    assert!(accepted.is_err(), "the probe never connected");
}
