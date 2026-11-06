//! D-3, D-6, WEB-007: device web views go only to `http` and `https`, and `https` opens only
//! where the platform can pin.

use tauri::Url;

use crate::web_view::{
    Fingerprint, OriginError, PinnedOrigin, WebUrl, WebViewError,
    address::{Embedding, embedding, is_web},
    pinning_available,
};

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn fingerprint() -> Fingerprint {
    "ab".repeat(32).parse().unwrap()
}

#[test]
fn web_007_the_filter_allows_http_and_https_only() {
    assert!(is_web(&url("http://192.0.2.7/")));
    assert!(is_web(&url("https://router.lan:8443/admin")));
    for refused in [
        "file:///etc/hosts",
        "data:text/html,x",
        "tauri://localhost/index.html",
        "ipc://localhost/web_view_open",
        "javascript:alert(1)",
        "about:blank",
        "blob:http://192.0.2.7/1",
        "ftp://192.0.2.7/",
        "anima-od7:probe",
        "asset://localhost/x",
    ] {
        assert!(!is_web(&url(refused)), "{refused}");
    }
}

#[test]
fn web_004_an_address_is_http_or_https_with_a_host_and_no_userinfo() {
    assert_eq!(
        WebUrl::parse("http://192.0.2.7/").unwrap().as_str(),
        "http://192.0.2.7/"
    );
    assert!(WebUrl::parse("HTTPS://Router.LAN:8443").unwrap().is_https());
    let cases = [
        ("not a url", WebViewError::Url),
        ("", WebViewError::Url),
        ("file:///etc/hosts", WebViewError::Scheme),
        ("javascript:alert(1)", WebViewError::Scheme),
        ("tauri://localhost/", WebViewError::Scheme),
        ("http://admin:secret@192.0.2.7/", WebViewError::Userinfo),
        ("https://admin@192.0.2.7/", WebViewError::Userinfo),
    ];
    for (text, error) in cases {
        assert_eq!(WebUrl::parse(text), Err(error), "{text}");
    }
}

#[test]
fn web_004_an_address_longer_than_8192_characters_is_refused() {
    let long = format!("http://192.0.2.7/{}", "a".repeat(8192));
    assert_eq!(WebUrl::parse(&long), Err(WebViewError::Url));
    let fits = format!("http://192.0.2.7/{}", "a".repeat(8192 - 17));
    assert!(WebUrl::parse(&fits).is_ok());
}

#[test]
fn web_004_an_address_is_written_back_normalised_so_it_starts_with_its_scheme() {
    let parsed = WebUrl::parse("  http://192.0.2.7/a b").unwrap();
    assert_eq!(parsed.as_str(), "http://192.0.2.7/a%20b");
}

#[test]
fn d_6_http_is_embedded_without_a_pin_on_every_platform() {
    let http = WebUrl::parse("http://192.0.2.7/").unwrap();
    assert_eq!(embedding(&http, None, false), Ok(Embedding::Plain));
    assert_eq!(embedding(&http, None, true), Ok(Embedding::Plain));
}

#[test]
fn d_6_a_pin_on_http_is_refused() {
    let http = WebUrl::parse("http://192.0.2.7/").unwrap();
    assert_eq!(
        embedding(&http, Some(fingerprint()), true),
        Err(WebViewError::PinScheme)
    );
}

#[test]
fn d_6_https_is_refused_where_the_platform_cannot_pin() {
    let https = WebUrl::parse("https://192.0.2.7/").unwrap();
    for pin in [None, Some(fingerprint())] {
        assert_eq!(
            embedding(&https, pin, false),
            Err(WebViewError::PinningUnavailable)
        );
    }
}

#[test]
fn d_6_https_is_embedded_with_its_origin_and_pin_where_the_platform_pins() {
    let https = WebUrl::parse("https://router.lan:8443/admin").unwrap();
    let origin = PinnedOrigin::parse("https://router.lan:8443/").unwrap();
    let Ok(Embedding::Pinned { origin: got, pin }) = embedding(&https, Some(fingerprint()), true)
    else {
        panic!("https with pinning is embedded");
    };
    assert_eq!(got, origin);
    let pin = pin.unwrap();
    assert_eq!((pin.origin, pin.fingerprint), (origin, fingerprint()));
    assert!(matches!(
        embedding(&https, None, true),
        Ok(Embedding::Pinned { pin: None, .. })
    ));
}

#[test]
fn d_6_no_platform_pins_until_the_trust_hook_is_ruled_on() {
    assert!(!pinning_available());
}

#[test]
fn log_007_an_origin_error_keeps_its_own_code() {
    assert_eq!(WebViewError::from(OriginError::NotHttps).code(), "scheme");
    assert_eq!(WebViewError::from(OriginError::NoHost).code(), "host");
}
