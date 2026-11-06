//! WP-31 "Shell interface", LOG-007: the `web-view` event payloads are exactly the client's
//! shapes, and every error crosses as `{ code }` with a code the client reads.

use serde_json::json;
use tauri::Url;

use crate::web_view::{
    CertificateError, Fingerprint, OriginError, WebViewError,
    commands::LeafCertificate,
    event::{EVENT, Failure, MAIN, WebViewEvent},
    tabs::TabId,
};

fn fingerprint(byte: &str) -> Fingerprint {
    byte.repeat(32).parse().unwrap()
}

fn tab() -> TabId {
    TabId::parse("tab-1").unwrap()
}

#[test]
fn web_004_the_event_is_web_view_sent_to_main() {
    assert_eq!((EVENT, MAIN), ("web-view", "main"));
}

#[test]
fn web_004_page_loads_report_their_http_or_https_url() {
    let url = Url::parse("https://router.lan:8443/admin?x=1").unwrap();
    let loading = WebViewEvent::page(&tab(), &url, false).unwrap();
    assert_eq!(
        serde_json::to_value(loading).unwrap(),
        json!({ "tabId": "tab-1", "kind": "LOADING", "url": "https://router.lan:8443/admin?x=1" })
    );
    let loaded = WebViewEvent::page(&tab(), &url, true).unwrap();
    assert_eq!(serde_json::to_value(loaded).unwrap()["kind"], "LOADED");
}

#[test]
fn web_007_no_load_is_reported_for_the_blank_start_or_another_scheme() {
    for url in ["about:blank", "file:///etc/hosts", "tauri://localhost/"] {
        let url = Url::parse(url).unwrap();
        assert_eq!(WebViewEvent::page(&tab(), &url, true), None);
    }
}

#[test]
fn web_008_untrusted_and_mismatch_carry_fingerprints_in_the_hub_form() {
    let untrusted = WebViewEvent::Untrusted {
        tab_id: "tab-1".to_owned(),
        fingerprint_sha256: fingerprint("AB"),
    };
    assert_eq!(
        serde_json::to_value(untrusted).unwrap(),
        json!({ "tabId": "tab-1", "kind": "UNTRUSTED", "fingerprintSha256": "ab".repeat(32) })
    );
    let mismatch = WebViewEvent::Mismatch {
        tab_id: "tab-1".to_owned(),
        expected: fingerprint("01"),
        presented: fingerprint("02"),
    };
    assert_eq!(
        serde_json::to_value(mismatch).unwrap(),
        json!({
            "tabId": "tab-1",
            "kind": "MISMATCH",
            "expected": "01".repeat(32),
            "presented": "02".repeat(32),
        })
    );
}

#[test]
fn web_010_a_failure_is_a_short_code() {
    let failed = WebViewEvent::Failed {
        tab_id: "tab-1".to_owned(),
        reason: Failure::Navigate,
    };
    assert_eq!(
        serde_json::to_value(failed).unwrap(),
        json!({ "tabId": "tab-1", "kind": "FAILED", "reason": "navigate" })
    );
}

#[test]
fn d_9_the_certificate_answer_is_fingerprint_sha256() {
    let answer = LeafCertificate {
        fingerprint_sha256: fingerprint("ef"),
    };
    assert_eq!(
        serde_json::to_value(answer).unwrap(),
        json!({ "fingerprintSha256": "ef".repeat(32) })
    );
}

/// The client keeps a code only if it matches `^[a-z][a-z0-9_]{0,39}$` (`codeOf`).
fn is_client_code(code: &str) -> bool {
    let mut bytes = code.bytes();
    code.len() <= 40
        && bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[test]
fn log_007_every_error_crosses_as_a_code_the_client_reads() {
    let errors = [
        WebViewError::Caller,
        WebViewError::TabId,
        WebViewError::DeviceId,
        WebViewError::Url,
        WebViewError::Scheme,
        WebViewError::Userinfo,
        WebViewError::Host,
        WebViewError::Arguments,
        WebViewError::Bounds,
        WebViewError::Step,
        WebViewError::Fingerprint,
        WebViewError::PinScheme,
        WebViewError::PinningUnavailable,
        WebViewError::TabOpen,
        WebViewError::TabLimit,
        WebViewError::UnknownTab,
        WebViewError::Window,
        WebViewError::Store,
        WebViewError::Native,
        WebViewError::Origin(OriginError::Userinfo),
        WebViewError::Certificate(CertificateError::Address),
        WebViewError::Certificate(CertificateError::Unreachable),
        WebViewError::Certificate(CertificateError::Handshake),
        WebViewError::Certificate(CertificateError::NoCertificate),
        WebViewError::Certificate(CertificateError::Timeout),
    ];
    for error in errors {
        let code = error.code();
        assert!(is_client_code(code), "{code}");
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "code": code })
        );
    }
    assert_eq!(
        WebViewError::PinningUnavailable.code(),
        "pinning_unavailable"
    );
    assert_eq!(WebViewError::Fingerprint.code(), "fingerprint");
}
