//! The settings of the proxy: the timeouts and the caps before the name have defaults, and 0 is
//! refused (SF-NEW-revroutelimits).

use std::{collections::BTreeMap, time::Duration};

use crate::settings::{SettingsError, from_env};

fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = [
        ("ANIMA_ROUTE_PROXY_LISTEN", "0.0.0.0:443"),
        ("ANIMA_ROUTE_PROXY_HUB_SOCKET", "/run/anima/routes.sock"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
    for (name, value) in pairs {
        env.insert((*name).to_owned(), (*value).to_owned());
    }
    env
}

#[test]
fn the_timeouts_and_the_caps_have_defaults() {
    let settings = from_env(&env(&[])).unwrap().unwrap();
    assert_eq!(settings.timeouts.handshake, Duration::from_secs(10));
    assert_eq!(settings.timeouts.head, Duration::from_secs(10));
    assert_eq!(settings.timeouts.request, Duration::from_secs(60));
    assert_eq!(settings.timeouts.idle, Duration::from_secs(300));
    assert_eq!(settings.tls_caps.total, 1024);
    assert!(
        settings.tls_caps.per_client < 32,
        "one client holds fewer streams than one link of the hub"
    );
    assert_eq!(settings.tls_caps.per_client, 8);
    assert_eq!(settings.door_addresses, Vec::<std::net::IpAddr>::new());
    assert_eq!(settings.plain_caps.total, 256);
}

#[test]
fn a_timeout_or_a_cap_of_zero_is_refused() {
    let idle = from_env(&env(&[("ANIMA_ROUTE_PROXY_IDLE_SECONDS", "0")]));
    assert_eq!(
        idle,
        Err(SettingsError::Invalid("ANIMA_ROUTE_PROXY_IDLE_SECONDS"))
    );
    let total = from_env(&env(&[("ANIMA_ROUTE_PROXY_OPEN_TOTAL", "0")]));
    assert_eq!(
        total,
        Err(SettingsError::Invalid("ANIMA_ROUTE_PROXY_OPEN_TOTAL"))
    );
    let set = from_env(&env(&[("ANIMA_ROUTE_PROXY_HEAD_SECONDS", "3")]))
        .unwrap()
        .unwrap();
    assert_eq!(set.timeouts.head, Duration::from_secs(3));
}

#[test]
fn the_addresses_of_the_door_are_read_and_a_bad_one_is_refused() {
    let set = from_env(&env(&[
        ("ANIMA_ROUTE_PROXY_DOOR", "true"),
        ("ANIMA_ROUTE_PROXY_DOOR_ADDRESSES", "10.0.0.5, 2001:db8::5"),
    ]))
    .unwrap()
    .unwrap();
    assert_eq!(
        set.door_addresses,
        vec![
            "10.0.0.5".parse::<std::net::IpAddr>().unwrap(),
            "2001:db8::5".parse().unwrap()
        ]
    );
    let bad = from_env(&env(&[("ANIMA_ROUTE_PROXY_DOOR_ADDRESSES", "door")]));
    assert_eq!(
        bad,
        Err(SettingsError::Invalid("ANIMA_ROUTE_PROXY_DOOR_ADDRESSES"))
    );
}
