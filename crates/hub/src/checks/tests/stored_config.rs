//! The stored JSON form of a check configuration: camelCase keys, the API's enum names, and
//! no unknown keys (CHK-009).

use crate::checks::config::{CheckConfig, HttpConfig, IpVersion};

fn http(edit: impl FnOnce(&mut HttpConfig)) -> CheckConfig {
    let mut config = HttpConfig::new("https://example.test/health");
    edit(&mut config);
    CheckConfig::Http(config)
}

#[test]
fn chk_009_stored_json_uses_camel_case_keys_and_the_api_enum_names() {
    let icmp = CheckConfig::Icmp {
        timeout_ms: 2000,
        ip_version: IpVersion::V6,
    };
    assert_eq!(
        serde_json::to_value(&icmp).unwrap(),
        serde_json::json!({"type": "icmp", "timeoutMs": 2000, "ipVersion": "V6"})
    );
    let http = serde_json::to_value(http(|c| c.verify_tls = false)).unwrap();
    assert_eq!(http["verifyTls"], false);
    assert_eq!(http["method"], "GET");
    assert_eq!(http["auth"], serde_json::json!({"kind": "NONE"}));
}

#[test]
fn chk_009_unknown_keys_are_refused_instead_of_defaulted() {
    let refused = [
        r#"{"type":"icmp","timeout_ms":2000}"#,
        r#"{"type":"tcp","port":22,"timeoutMs":5000,"extra":1}"#,
        r#"{"type":"http","url":"http://a.test/","verify_tls":false}"#,
        r#"{"type":"http","url":"http://a.test/","auth":{"kind":"BASIC","username":"u","password":"p"}}"#,
        r#"{"type":"http","url":"http://a.test/","headers":[{"name":"A","value":"b","secret":"c"}]}"#,
        r#"{"type":"icmp","ipVersion":"v4"}"#,
    ];
    for json in refused {
        assert!(serde_json::from_str::<CheckConfig>(json).is_err(), "{json}");
    }
    let accepted: CheckConfig =
        serde_json::from_str(r#"{"type":"http","url":"http://a.test/","verifyTls":false}"#)
            .unwrap();
    let CheckConfig::Http(accepted) = accepted else {
        panic!("not http")
    };
    assert!(!accepted.verify_tls);
}
