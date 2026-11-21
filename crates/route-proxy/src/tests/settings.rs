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
    assert_eq!(settings.timeouts.idle, Duration::from_secs(300));
    assert_eq!(settings.tls_caps.total, 1024);
    assert_eq!(settings.tls_caps.per_client, 32);
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
