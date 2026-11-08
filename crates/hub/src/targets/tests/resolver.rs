//! The checked resolver (P3-003): a name is refused when any of its addresses is, and the
//! caller gets only checked addresses in canonical form.

use std::net::SocketAddr;

use crate::targets::{AddressPolicy, TargetError, every_permitted, resolve_checked};

fn socket(text: &str) -> SocketAddr {
    SocketAddr::new(text.parse().unwrap(), 22)
}

#[test]
fn r_032_a_name_is_refused_when_any_of_its_addresses_is() {
    let public = vec![socket("10.10.0.2"), socket("2001:db8::1")];
    let policy = AddressPolicy::default();
    assert_eq!(every_permitted(&policy, public.clone()), Ok(public));
    for mixed in [
        vec![socket("10.10.0.2"), socket("127.0.0.1")],
        vec![socket("::1"), socket("10.10.0.2")],
        vec![
            socket("10.10.0.2"),
            socket("169.254.169.254"),
            socket("10.10.0.3"),
        ],
    ] {
        assert_eq!(
            every_permitted(&policy, mixed.clone()),
            Err(TargetError::Refused),
            "{mixed:?}"
        );
    }
    assert_eq!(
        every_permitted(&policy, Vec::new()),
        Err(TargetError::Unresolved)
    );
}

#[test]
fn d_1_one_denied_address_refuses_the_name_whichever_comes_first() {
    let policy = AddressPolicy::new(vec!["10.10.0.0/24".parse().unwrap()]);
    for addresses in [
        vec![socket("10.20.0.5"), socket("10.10.0.9")],
        vec![socket("10.10.0.9"), socket("10.20.0.5")],
        vec![socket("10.20.0.5"), socket("169.254.169.254")],
    ] {
        assert_eq!(
            every_permitted(&policy, addresses.clone()),
            Err(TargetError::Refused),
            "{addresses:?}"
        );
    }
}

#[tokio::test]
async fn p3_003_a_literal_resolves_to_itself_in_canonical_form_with_the_port() {
    let policy = AddressPolicy::default();
    assert_eq!(
        resolve_checked("10.10.0.21", 3389, &policy).await,
        Ok(vec![SocketAddr::new("10.10.0.21".parse().unwrap(), 3389)])
    );
    assert_eq!(
        resolve_checked("::ffff:10.10.0.21", 22, &policy).await,
        Ok(vec![socket("10.10.0.21")]),
        "an IPv4-mapped address is the IPv4 address it is"
    );
}

#[tokio::test]
async fn p3_003_a_refused_or_unresolved_host_gives_no_address() {
    let policy = AddressPolicy::default();
    for host in [
        "localhost",
        "127.0.0.1",
        "::ffff:127.0.0.1",
        "169.254.169.254",
        "64:ff9b:1::1",
    ] {
        assert_eq!(
            resolve_checked(host, 22, &policy).await,
            Err(TargetError::Refused),
            "{host}"
        );
    }
    let denying = AddressPolicy::new(vec!["10.10.0.0/24".parse().unwrap()]);
    assert_eq!(
        resolve_checked("10.10.0.21", 22, &denying).await,
        Err(TargetError::Refused)
    );
    assert_eq!(
        resolve_checked("no-such-device.invalid", 22, &policy).await,
        Err(TargetError::Unresolved)
    );
    assert_eq!(
        resolve_checked("[::1]", 22, &policy).await,
        Err(TargetError::Unresolved),
        "no brackets"
    );
}
