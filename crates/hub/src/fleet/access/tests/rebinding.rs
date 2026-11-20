//! The link mode's names: a loopback host literal and no origin or a client origin
//! (FRN-007). The layer itself is tested in the `it` crate (`wiring/link/rebinding.rs`).

use axum::http::{HeaderMap, HeaderValue, header::ORIGIN};
use proptest::prelude::*;

use crate::fleet::access::rebinding::{is_host_of, is_loopback_host, origin_permitted};

#[test]
fn frn_007_only_loopback_literals_with_a_numeric_port_are_hosts() {
    for host in [
        "127.0.0.1",
        "127.0.0.1:8080",
        "localhost",
        "LocalHost:1",
        "[::1]",
        "[::1]:65535",
    ] {
        assert!(is_loopback_host(host), "{host}");
    }
    for host in [
        "",
        ":80",
        "127.0.0.1:",
        "127.0.0.1:123456",
        "127.0.0.1:8o",
        "127.0.0.1:80:80",
        "127.0.0.2",
        "127.1",
        "2130706433",
        "localhost.",
        "localhost.rebind.example",
        "rebind.example",
        "127.0.0.1.rebind.example",
        "[::1",
        "[::1]x",
        "[::1]:",
        "[0:0:0:0:0:0:0:1]",
        "[::ffff:127.0.0.1]",
        "::1",
        "user@127.0.0.1",
    ] {
        assert!(!is_loopback_host(host), "{host}");
    }
}

const CLIENTS: &[&str] = &["tauri://localhost", "http://tauri.localhost"];

fn with_origins(origins: &[&[u8]]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for origin in origins {
        headers.append(ORIGIN, HeaderValue::from_bytes(origin).unwrap());
    }
    headers
}

#[test]
fn frn_007_no_origin_or_only_client_origins_pass() {
    assert!(origin_permitted(&with_origins(&[]), CLIENTS, None));
    assert!(origin_permitted(
        &with_origins(&[b"tauri://localhost"]),
        CLIENTS,
        None
    ));
    for refused in [
        &[&b"http://127.0.0.1:8080"[..]][..],
        &[b"null"],
        &[b"tauri://localhost.rebind.example"],
        &[b"TAURI://LOCALHOST"],
        &[b"tauri://localhost", b"http://rebind.example"],
        &[b"tauri://localhost\xff"],
    ] {
        assert!(
            !origin_permitted(&with_origins(refused), CLIENTS, None),
            "{refused:?}"
        );
    }
}

proptest! {
    /// Any text: no panic, and a host that passes is one of the three names.
    #[test]
    fn frn_007_any_host_text_is_safe(host in any::<String>()) {
        if is_loopback_host(&host) {
            let name = host.rsplit_once(':').filter(|_| !host.ends_with(']')).map_or(host.as_str(), |(name, _)| name);
            prop_assert!(["127.0.0.1", "[::1]"].contains(&name) || name.eq_ignore_ascii_case("localhost"));
        }
    }
}

const GATEWAY: Option<&str> = Some("https://hub.anima.ing");

#[test]
fn hmd_061_a_hosted_hub_also_answers_the_name_and_the_origin_of_its_gateway() {
    for named in ["hub.anima.ing", "HUB.anima.ing", "hub.anima.ing:443"] {
        assert!(is_host_of(named, GATEWAY), "{named}");
    }
    for refused in [
        "hub.anima.ing:8443",
        "evil.hub.anima.ing",
        "hub.anima.ing.evil.example",
        "anima.ing",
        "",
    ] {
        assert!(!is_host_of(refused, GATEWAY), "{refused}");
    }
    assert!(origin_permitted(
        &with_origins(&[b"https://hub.anima.ing"]),
        CLIENTS,
        GATEWAY
    ));
    assert!(!origin_permitted(
        &with_origins(&[b"https://evil.example"]),
        CLIENTS,
        GATEWAY
    ));
}

#[test]
fn hmd_061_a_hub_that_is_not_hosted_answers_no_other_name_and_no_other_origin() {
    assert!(!is_host_of("hub.anima.ing", None));
    assert!(!is_host_of("", Some("https://")));
    assert!(!is_host_of("hub.anima.ing", Some("http://hub.anima.ing")));
    assert!(!origin_permitted(
        &with_origins(&[b"https://hub.anima.ing"]),
        CLIENTS,
        None
    ));
}
