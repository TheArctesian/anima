//! Docker and systemd configuration: ids, stored JSON, defaults and name rules (CHK-009,
//! CHK-012, D-3).

use crate::checks::config::{CheckConfig, CheckType, ConfigError, Reason};

fn docker(container: &str, timeout_ms: u32) -> CheckConfig {
    CheckConfig::Docker {
        container: container.to_owned(),
        timeout_ms,
    }
}

fn systemd(unit: &str, timeout_ms: u32) -> CheckConfig {
    CheckConfig::Systemd {
        unit: unit.to_owned(),
        timeout_ms,
    }
}

fn error(config: &CheckConfig) -> Option<(&'static str, Reason)> {
    config
        .validate()
        .err()
        .map(|ConfigError { field, reason }| (field, reason))
}

#[test]
fn chk_009_docker_and_systemd_ids_are_fixed_and_round_trip() {
    for (kind, id) in [
        (CheckType::Docker, "docker"),
        (CheckType::Systemd, "systemd"),
    ] {
        assert_eq!(kind.as_sql(), id);
        assert_eq!(CheckType::from_sql(id), Some(kind));
    }
    assert_eq!(docker("web", 10_000).kind(), CheckType::Docker);
    assert_eq!(systemd("nginx", 10_000).kind(), CheckType::Systemd);
}

#[test]
fn chk_009_stored_json_is_tagged_with_the_id_and_uses_camel_case() {
    assert_eq!(
        serde_json::to_value(docker("web", 5000)).unwrap(),
        serde_json::json!({"type": "docker", "container": "web", "timeoutMs": 5000})
    );
    assert_eq!(
        serde_json::to_value(systemd("nginx.service", 8000)).unwrap(),
        serde_json::json!({"type": "systemd", "unit": "nginx.service", "timeoutMs": 8000})
    );
    for config in [docker("web-1", 5000), systemd("nginx.service", 8000)] {
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(serde_json::from_str::<CheckConfig>(&json).unwrap(), config);
    }
}

#[test]
fn chk_009_a_missing_timeout_is_10_seconds_and_unknown_keys_are_refused() {
    let docker_json = r#"{"type":"docker","container":"web"}"#;
    let systemd_json = r#"{"type":"systemd","unit":"nginx"}"#;
    assert_eq!(
        serde_json::from_str::<CheckConfig>(docker_json).unwrap(),
        docker("web", 10_000)
    );
    assert_eq!(
        serde_json::from_str::<CheckConfig>(systemd_json).unwrap(),
        systemd("nginx", 10_000)
    );
    for refused in [
        r#"{"type":"docker","container":"web","timeout_ms":5000}"#,
        r#"{"type":"docker","timeoutMs":5000}"#,
        r#"{"type":"systemd","unit":"nginx","extra":1}"#,
        r#"{"type":"systemd","timeoutMs":5000}"#,
    ] {
        assert!(
            serde_json::from_str::<CheckConfig>(refused).is_err(),
            "{refused}"
        );
    }
}

#[test]
fn chk_012_the_probe_time_limit_is_the_configured_timeout() {
    let limit = |config: &CheckConfig| config.timeout().as_millis();
    assert_eq!(limit(&docker("web", 7000)), 7000);
    assert_eq!(limit(&systemd("nginx", 9000)), 9000);
}

#[test]
fn chk_012_container_names_start_with_a_letter_or_digit_and_hold_only_name_characters() {
    for name in ["web", "0web", "a", "web-1", "web_1.v2", "A-B_c.d", "9"] {
        assert_eq!(error(&docker(name, 10_000)), None, "{name}");
    }
    for name in [
        "", "-web", ".web", "_web", "we b", "web;rm", "web/x", "web,db", "we\nb", "wéb", " web",
        "web ", "$(id)", "web'x", "web\"x", "web\\x", "web:x", "web@x",
    ] {
        let found = error(&docker(name, 10_000)).map(|(field, _)| field);
        assert_eq!(found, Some("container"), "{name:?}");
    }
    assert_eq!(
        error(&docker("", 10_000)),
        Some(("container", Reason::Empty))
    );
    assert_eq!(
        error(&docker("-web", 10_000)),
        Some(("container", Reason::Invalid))
    );
}

#[test]
fn chk_012_unit_names_hold_only_unit_characters_and_never_start_with_a_dash() {
    for name in [
        "nginx",
        "nginx.service",
        "postgresql@16-main.service",
        "getty@tty1",
        "user-1000.slice",
        "dev-sda1.device",
        "run-media:disk.mount",
        "_x",
        ".x",
        "@x",
        ":x",
        "\\x2dthing.service",
        "a-b-c",
    ] {
        assert_eq!(error(&systemd(name, 10_000)), None, "{name}");
    }
    for name in [
        "", "-nginx", "--now", "ng inx", "nginx;id", "nginx/x", "nginx,x", "ng\nx", "ngïnx",
        " nginx", "nginx ", "$(id)", "nginx'x", "nginx*",
    ] {
        let found = error(&systemd(name, 10_000)).map(|(field, _)| field);
        assert_eq!(found, Some("unit"), "{name:?}");
    }
    assert_eq!(error(&systemd("", 10_000)), Some(("unit", Reason::Empty)));
    assert_eq!(
        error(&systemd("-x", 10_000)),
        Some(("unit", Reason::Invalid))
    );
}

#[test]
fn chk_012_names_are_at_most_255_bytes() {
    let longest = "a".repeat(255);
    let too_long = "a".repeat(256);
    assert_eq!(error(&docker(&longest, 10_000)), None);
    assert_eq!(error(&systemd(&longest, 10_000)), None);
    assert_eq!(
        error(&docker(&too_long, 10_000)),
        Some(("container", Reason::TooLong { max: 255 }))
    );
    assert_eq!(
        error(&systemd(&too_long, 10_000)),
        Some(("unit", Reason::TooLong { max: 255 }))
    );
}

#[test]
fn chk_009_timeouts_are_100_through_60000_ms_and_the_error_names_the_field_only() {
    for ms in [100, 10_000, 60_000] {
        assert_eq!(error(&docker("web", ms)), None, "{ms}");
        assert_eq!(error(&systemd("nginx", ms)), None, "{ms}");
    }
    for ms in [0, 99, 60_001] {
        let range = Reason::OutOfRange {
            min: 100,
            max: 60_000,
        };
        assert_eq!(
            error(&docker("web", ms)),
            Some(("timeoutMs", range)),
            "{ms}"
        );
        assert_eq!(
            error(&systemd("nginx", ms)),
            Some(("timeoutMs", range)),
            "{ms}"
        );
    }
    let refused = docker("bad name", 10_000)
        .validate()
        .unwrap_err()
        .to_string();
    assert_eq!(refused, "container: is not valid");
}
