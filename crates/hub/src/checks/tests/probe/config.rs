//! Configuration: fixed ids, stored JSON shape and validation (CHK-009).

use crate::checks::config::{
    CheckConfig, CheckType, ConfigError, DnsRecordType, HttpConfig, HttpHeader, IpVersion, Reason,
    StatusRange,
};

fn tcp(port: u16, timeout_ms: u32) -> CheckConfig {
    CheckConfig::Tcp { port, timeout_ms }
}

fn http(edit: impl FnOnce(&mut HttpConfig)) -> CheckConfig {
    let mut config = HttpConfig::new("https://example.test/health");
    edit(&mut config);
    CheckConfig::Http(config)
}

fn field(config: &CheckConfig) -> Option<&'static str> {
    config
        .validate()
        .err()
        .map(|ConfigError { field, .. }| field)
}

#[test]
fn chk_009_type_ids_are_fixed_and_round_trip() {
    let ids = [
        (CheckType::Icmp, "icmp"),
        (CheckType::Tcp, "tcp"),
        (CheckType::Tls, "tls"),
        (CheckType::Http, "http"),
        (CheckType::Dns, "dns"),
        (CheckType::Push, "push"),
        (CheckType::Group, "group"),
    ];
    for (kind, id) in ids {
        assert_eq!(kind.as_sql(), id);
        assert_eq!(CheckType::from_sql(id), Some(kind));
    }
    assert_eq!(CheckType::from_sql("ping"), None);
    assert_eq!(CheckType::from_sql("TCP"), None);
}

#[test]
fn chk_009_stored_json_is_tagged_with_the_type_id() {
    let json = serde_json::to_value(tcp(22, 5000)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"type": "tcp", "port": 22, "timeoutMs": 5000})
    );
    assert_eq!(
        serde_json::to_value(CheckConfig::Push).unwrap(),
        serde_json::json!({"type": "push"})
    );
    let kinds = [CheckConfig::Push, CheckConfig::Group, tcp(1, 100)];
    for config in kinds {
        let back: CheckConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(back, config);
        let tag = serde_json::to_value(&config).unwrap()["type"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(tag, config.kind().as_sql());
    }
}

#[test]
fn chk_009_contract_defaults_apply_to_missing_fields() {
    let icmp: CheckConfig = serde_json::from_str(r#"{"type":"icmp"}"#).unwrap();
    assert_eq!(
        icmp,
        CheckConfig::Icmp {
            timeout_ms: 2000,
            ip_version: IpVersion::V4
        }
    );
    let tls: CheckConfig = serde_json::from_str(r#"{"type":"tls"}"#).unwrap();
    assert_eq!(
        tls,
        CheckConfig::Tls {
            port: 443,
            server_name: None,
            timeout_ms: 5000
        }
    );
    let dns: CheckConfig = serde_json::from_str(r#"{"type":"dns","name":"a.test"}"#).unwrap();
    assert_eq!(
        dns,
        CheckConfig::Dns {
            name: "a.test".into(),
            record_type: DnsRecordType::A,
            resolver: None,
            expected: None
        }
    );
    let http: CheckConfig =
        serde_json::from_str(r#"{"type":"http","url":"http://a.test/"}"#).unwrap();
    let CheckConfig::Http(http) = http else {
        panic!("not http")
    };
    assert_eq!(
        (http.max_redirects, http.timeout_ms, http.verify_tls),
        (10, 10_000, true)
    );
    assert_eq!(
        http.accepted_status,
        vec![StatusRange { from: 200, to: 299 }]
    );
}

#[test]
fn timeouts_are_limited_to_100_through_60000_ms() {
    assert_eq!(field(&tcp(80, 99)), Some("timeoutMs"));
    assert_eq!(field(&tcp(80, 100)), None);
    assert_eq!(field(&tcp(80, 60_000)), None);
    assert_eq!(field(&tcp(80, 60_001)), Some("timeoutMs"));
    assert_eq!(field(&http(|c| c.timeout_ms = 99)), Some("timeoutMs"));
    assert_eq!(field(&http(|c| c.timeout_ms = 60_001)), Some("timeoutMs"));
    let icmp = CheckConfig::Icmp {
        timeout_ms: 60_001,
        ip_version: IpVersion::V4,
    };
    assert_eq!(field(&icmp), Some("timeoutMs"));
}

#[test]
fn ports_are_limited_to_1_through_65535() {
    assert_eq!(field(&tcp(0, 1000)), Some("port"));
    assert_eq!(field(&tcp(1, 1000)), None);
    assert_eq!(field(&tcp(65_535, 1000)), None);
    let tls = CheckConfig::Tls {
        port: 0,
        server_name: None,
        timeout_ms: 1000,
    };
    assert_eq!(field(&tls), Some("port"));
}

#[test]
fn urls_must_be_short_http_or_https_without_userinfo() {
    let url = |text: &str| http(|c| c.url = text.to_owned());
    assert_eq!(field(&url("http://a.test/")), None);
    assert_eq!(field(&url("ftp://a.test/")), Some("url"));
    assert_eq!(field(&url("file:///etc/passwd")), Some("url"));
    assert_eq!(field(&url("not a url")), Some("url"));
    assert_eq!(field(&url("http://user:pw@a.test/")), Some("url"));
    assert_eq!(field(&url("http://user@a.test/")), Some("url"));
    let long_ok = format!(
        "http://a.test/{}",
        "a".repeat(2048 - "http://a.test/".len())
    );
    assert_eq!(field(&url(&long_ok)), None);
    assert_eq!(field(&url(&format!("{long_ok}a"))), Some("url"));
    let err = url("ftp://user:topsecret@a.test/");
    let ConfigError { reason, .. } = err.validate().unwrap_err();
    assert!(
        !err.validate()
            .unwrap_err()
            .to_string()
            .contains("topsecret")
    );
    assert_eq!(reason, Reason::Scheme);
}

#[test]
fn headers_are_tokens_with_clean_values_and_at_most_20() {
    let header = |name: &str, value: &str| HttpHeader {
        name: name.into(),
        value: Some(value.into()),
    };
    let with = |h: Vec<HttpHeader>| http(|c| c.headers = h);
    assert_eq!(field(&with(vec![header("X-Check", "ok")])), None);
    assert_eq!(field(&with(vec![header("X Check", "ok")])), Some("headers"));
    assert_eq!(field(&with(vec![header("", "ok")])), Some("headers"));
    assert_eq!(
        field(&with(vec![header("X-A", "a\r\nX-B: b")])),
        Some("headers")
    );
    assert_eq!(field(&with(vec![header("X-A", "a\nb")])), Some("headers"));
    assert_eq!(field(&with(vec![header("X-A", "a\0b")])), Some("headers"));
    let twenty = (0..20)
        .map(|i| header(&format!("X-{i}"), "v"))
        .collect::<Vec<_>>();
    assert_eq!(field(&with(twenty.clone())), None);
    let mut twenty_one = twenty;
    twenty_one.push(header("X-20", "v"));
    assert_eq!(field(&with(twenty_one)), Some("headers"));
}

#[test]
fn secret_header_values_are_refused_in_the_stored_form() {
    for name in ["Authorization", "proxy-authorization", "COOKIE"] {
        let inline = HttpHeader {
            name: name.into(),
            value: Some("v".into()),
        };
        assert_eq!(
            field(&http(|c| c.headers = vec![inline])),
            Some("headers"),
            "{name}"
        );
        let held = HttpHeader {
            name: name.into(),
            value: None,
        };
        assert_eq!(field(&http(|c| c.headers = vec![held])), None, "{name}");
    }
    let plain_without_value = HttpHeader {
        name: "X-A".into(),
        value: None,
    };
    assert_eq!(
        field(&http(|c| c.headers = vec![plain_without_value])),
        Some("headers")
    );
}

#[test]
fn body_is_limited_to_64_kib_and_redirects_to_10() {
    assert_eq!(field(&http(|c| c.body = Some("a".repeat(64 * 1024)))), None);
    assert_eq!(
        field(&http(|c| c.body = Some("a".repeat(64 * 1024 + 1)))),
        Some("body")
    );
    assert_eq!(field(&http(|c| c.max_redirects = 10)), None);
    assert_eq!(field(&http(|c| c.max_redirects = 11)), Some("maxRedirects"));
}

#[test]
fn status_ranges_lie_in_100_through_599_and_run_upwards() {
    let ranges = |r: Vec<(u16, u16)>| {
        http(|c| {
            c.accepted_status = r
                .into_iter()
                .map(|(from, to)| StatusRange { from, to })
                .collect();
        })
    };
    assert_eq!(field(&ranges(vec![(100, 599)])), None);
    assert_eq!(field(&ranges(vec![(200, 200)])), None);
    assert_eq!(field(&ranges(vec![(99, 200)])), Some("acceptedStatus"));
    assert_eq!(field(&ranges(vec![(200, 600)])), Some("acceptedStatus"));
    assert_eq!(field(&ranges(vec![(300, 200)])), Some("acceptedStatus"));
    assert_eq!(field(&ranges(vec![])), Some("acceptedStatus"));
}

#[test]
fn keyword_and_json_query_are_bounded_and_the_query_is_a_pointer() {
    assert_eq!(field(&http(|c| c.keyword = Some("k".repeat(1024)))), None);
    assert_eq!(
        field(&http(|c| c.keyword = Some("k".repeat(1025)))),
        Some("keyword")
    );
    let query = |q: &str| http(|c| c.json_query = Some(q.to_owned()));
    for good in ["", "/a", "/a/0/b", "/a~0b/c~1d", "/"] {
        assert_eq!(field(&query(good)), None, "{good}");
    }
    for bad in ["a", "a/b", "/a~", "/a~2", "$.a"] {
        assert_eq!(field(&query(bad)), Some("jsonQuery"), "{bad}");
    }
    let at_limit = format!("/{}", "a".repeat(511));
    assert_eq!(field(&query(&at_limit)), None);
    assert_eq!(field(&query(&format!("{at_limit}a"))), Some("jsonQuery"));
    assert_eq!(
        field(&http(|c| c.json_expected = Some("x".into()))),
        Some("jsonExpected")
    );
}

#[test]
fn dns_and_server_names_are_valid_names_of_at_most_253_bytes() {
    let dns = |name: &str| CheckConfig::Dns {
        name: name.into(),
        record_type: DnsRecordType::A,
        resolver: None,
        expected: None,
    };
    for good in [
        "example.test",
        "a-b.example.test.",
        "_sip._tcp.example.test",
        "1.0.0.127.in-addr.arpa",
    ] {
        assert_eq!(field(&dns(good)), None, "{good}");
    }
    for bad in [
        "",
        ".",
        "a..b",
        "-a.test",
        "a-.test",
        "a b.test",
        "ex\u{e4}mple.test",
        &"a".repeat(64),
    ] {
        assert_eq!(field(&dns(bad)), Some("name"), "{bad}");
    }
    let label = "a".repeat(63);
    let max = format!("{label}.{label}.{label}.{}", "a".repeat(61));
    assert_eq!(max.len(), 253);
    assert_eq!(field(&dns(&max)), None);
    assert_eq!(field(&dns(&format!("{max}a"))), Some("name"));
    let tls = |name: &str| CheckConfig::Tls {
        port: 443,
        server_name: Some(name.into()),
        timeout_ms: 1000,
    };
    assert_eq!(field(&tls("host.example.test")), None);
    assert_eq!(field(&tls("host_name.test")), Some("serverName"));
    assert_eq!(field(&tls("")), Some("serverName"));
}

#[test]
fn chk_009_stored_json_uses_camel_case_keys_and_the_k1_enum_names() {
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
