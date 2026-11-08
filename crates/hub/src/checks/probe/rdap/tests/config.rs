//! The domain expiry configuration: its fixed id, stored JSON, defaults and domain rules
//! (CHK-009, D-29).

use crate::checks::config::{CheckConfig, CheckType, ConfigError, Reason};

fn domain(name: &str) -> CheckConfig {
    CheckConfig::DomainExpiry {
        domain: name.to_owned(),
        warn_days: 30,
        timeout_ms: 10_000,
    }
}

fn error(config: &CheckConfig) -> Option<(&'static str, Reason)> {
    config
        .validate()
        .err()
        .map(|ConfigError { field, reason }| (field, reason))
}

#[test]
fn chk_009_the_domain_expiry_id_is_fixed_and_round_trips() {
    assert_eq!(CheckType::DomainExpiry.as_sql(), "domain_expiry");
    assert_eq!(
        CheckType::from_sql("domain_expiry"),
        Some(CheckType::DomainExpiry)
    );
    assert_eq!(domain("example.net").kind(), CheckType::DomainExpiry);
}

#[test]
fn chk_009_stored_json_uses_camel_case_and_fills_the_defaults() {
    assert_eq!(
        serde_json::to_value(domain("example.net")).unwrap(),
        serde_json::json!({
            "type": "domain_expiry", "domain": "example.net", "warnDays": 30, "timeoutMs": 10000
        })
    );
    let bare = r#"{"type":"domain_expiry","domain":"example.net"}"#;
    assert_eq!(
        serde_json::from_str::<CheckConfig>(bare).unwrap(),
        domain("example.net")
    );
    for refused in [
        r#"{"type":"domain_expiry"}"#,
        r#"{"type":"domain_expiry","domain":"example.net","warn_days":5}"#,
        r#"{"type":"domain_expiry","domain":"example.net","url":"https://x"}"#,
    ] {
        assert!(
            serde_json::from_str::<CheckConfig>(refused).is_err(),
            "{refused}"
        );
    }
}

#[test]
fn d_29_the_probe_time_limit_is_the_configured_timeout() {
    let config = CheckConfig::DomainExpiry {
        domain: "example.net".to_owned(),
        warn_days: 30,
        timeout_ms: 4000,
    };
    assert_eq!(config.timeout().as_millis(), 4000);
}

#[test]
fn d_29_a_domain_is_an_ascii_name_of_two_labels_or_more() {
    for name in [
        "example.net",
        "www.example.co.uk",
        "EXAMPLE.NET",
        "xn--bcher-kva.example",
        "example.xn--p1ai",
        "a-b.example",
        "1.example",
    ] {
        assert_eq!(error(&domain(name)), None, "{name}");
    }
}

#[test]
fn d_29_a_scheme_path_port_address_or_single_label_is_refused() {
    let not_domain = Some(("domain", Reason::NotDomain));
    for name in [
        "https://example.net",
        "example.net/",
        "example.net/whois",
        "example.net:443",
        "example.net.",
        "net",
        "localhost",
        "192.168.1.10",
        "[::1]",
        "example.123",
        "-example.net",
        "example..net",
        "exa mple.net",
        "user@example.net",
        "example.net?x",
        "_dmarc.example.net",
    ] {
        assert_eq!(error(&domain(name)), not_domain, "{name}");
    }
    assert_eq!(error(&domain("")), Some(("domain", Reason::Empty)));
    let long = format!("{}.net", "a.".repeat(130));
    assert_eq!(
        error(&domain(&long)),
        Some(("domain", Reason::TooLong { max: 253 }))
    );
}

#[test]
fn d_29_a_non_ascii_domain_asks_for_its_a_label_form() {
    for name in ["bücher.example", "example.рф", "exam\u{200b}ple.net"] {
        assert_eq!(
            error(&domain(name)),
            Some(("domain", Reason::NotAscii)),
            "{name}"
        );
    }
    assert_eq!(
        Reason::NotAscii.to_string(),
        "must be ASCII: write an internationalised domain in its xn-- form"
    );
}

#[test]
fn d_29_warning_days_and_timeout_have_bounds() {
    let with = |warn_days, timeout_ms| CheckConfig::DomainExpiry {
        domain: "example.net".to_owned(),
        warn_days,
        timeout_ms,
    };
    let days = Some(("warnDays", Reason::OutOfRange { min: 1, max: 365 }));
    let timeout = Some((
        "timeoutMs",
        Reason::OutOfRange {
            min: 100,
            max: 60_000,
        },
    ));
    assert_eq!(error(&with(1, 100)), None);
    assert_eq!(error(&with(365, 60_000)), None);
    assert_eq!(error(&with(0, 10_000)), days);
    assert_eq!(error(&with(366, 10_000)), days);
    assert_eq!(error(&with(30, 99)), timeout);
    assert_eq!(error(&with(30, 60_001)), timeout);
}
