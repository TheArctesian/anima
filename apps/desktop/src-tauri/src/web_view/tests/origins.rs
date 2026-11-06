//! H-4: only `https` takes a pin; the origin is scheme, host and port.

use crate::web_view::{OriginError, PinnedOrigin, pin::Host};

fn origin(url: &str) -> PinnedOrigin {
    PinnedOrigin::parse(url).unwrap()
}

#[test]
fn h_4_an_https_origin_defaults_to_port_443() {
    let parsed = origin("https://router.lan/admin");
    assert_eq!(parsed.host, Host::Name("router.lan".to_owned()));
    assert_eq!(parsed.port, 443);
    assert_eq!(origin("https://router.lan:443/"), parsed);
}

#[test]
fn h_4_an_explicit_port_is_kept() {
    let parsed = origin("https://router.lan:8443/");
    assert_eq!(parsed.port, 8443);
    assert_ne!(parsed, origin("https://router.lan/"));
}

#[test]
fn h_4_the_host_is_compared_without_case_or_a_trailing_dot() {
    assert_eq!(origin("https://Router.LAN/"), origin("https://router.lan/"));
    assert_eq!(
        origin("https://router.lan./"),
        origin("https://router.lan/")
    );
    assert_ne!(origin("https://router.lan/"), origin("https://router.la/"));
}

#[test]
fn h_4_a_path_query_or_fragment_does_not_change_the_origin() {
    assert_eq!(
        origin("https://router.lan/a?b=c#d"),
        origin("https://router.lan")
    );
}

#[test]
fn h_4_addresses_are_read_as_addresses() {
    assert_eq!(
        origin("https://192.168.1.1/").host,
        Host::Address([192, 168, 1, 1].into())
    );
    let v6 = origin("https://[FD00:0::1]:8006/");
    assert_eq!(v6.host, Host::Address("fd00::1".parse().unwrap()));
    assert_eq!(v6.port, 8006);
}

#[test]
fn h_4_only_https_takes_a_pin() {
    for url in [
        "http://router.lan/",
        "ftp://router.lan/",
        "wss://router.lan/",
        "file:///etc/hosts",
    ] {
        assert_eq!(
            PinnedOrigin::parse(url),
            Err(OriginError::NotHttps),
            "{url}"
        );
    }
}

#[test]
fn h_4_a_url_with_userinfo_takes_no_pin() {
    for url in [
        "https://admin@router.lan/",
        "https://admin:secret@router.lan/",
        "https://:secret@router.lan/",
    ] {
        assert_eq!(
            PinnedOrigin::parse(url),
            Err(OriginError::Userinfo),
            "{url}"
        );
    }
}

#[test]
fn h_4_text_that_is_no_url_or_has_no_host_takes_no_pin() {
    for url in [
        "",
        "router.lan",
        "https://",
        "https:///",
        "https://:443/",
        "https://[::1/",
    ] {
        assert!(PinnedOrigin::parse(url).is_err(), "{url}");
    }
    assert_eq!(
        PinnedOrigin::parse("router.lan"),
        Err(OriginError::Unreadable)
    );
}

#[test]
fn log_007_origin_errors_carry_only_a_code() {
    let codes = [
        (OriginError::Unreadable, "url"),
        (OriginError::NotHttps, "scheme"),
        (OriginError::Userinfo, "userinfo"),
        (OriginError::NoHost, "host"),
    ];
    for (error, code) in codes {
        assert_eq!(error.code(), code);
    }
    let error = PinnedOrigin::parse("https://admin:secret@router.lan/").unwrap_err();
    let text = error.to_string();
    assert!(
        !text.contains("secret") && !text.contains("router.lan"),
        "{text}"
    );
}
