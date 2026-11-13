//! The limits of the SNMP input fields, named as the client editor names them (D-5,
//! RFC 3414, LOG-007).

use super::rules::{AUTH, PRIVACY, config, field, input, v3};
use crate::fleet::checks::{
    config::{SnmpConfig, SnmpVersion},
    rules::{self, Stored},
};

#[test]
fn d_5_secret_lengths_follow_rfc_3414() {
    let long = "x".repeat(257);
    let v2c = config(SnmpVersion::V2c, None);
    let created = rules::prepare(
        input(v2c.clone(), Some(&"x".repeat(256)), None, None),
        Stored::Nothing,
    );
    assert!(created.is_ok());
    let refused = rules::prepare(input(v2c, Some(&long), None, None), Stored::Nothing);
    assert_eq!(field(refused), "community");
    assert_eq!(
        field(rules::prepare(
            v3(Some("seven77"), Some(PRIVACY)),
            Stored::Nothing
        )),
        "authPassword"
    );
    assert_eq!(
        field(rules::prepare(
            v3(Some(AUTH), Some("seven77")),
            Stored::Nothing
        )),
        "privacyPassword"
    );
    assert_eq!(
        field(rules::prepare(
            v3(Some(&long), Some(PRIVACY)),
            Stored::Nothing
        )),
        "authPassword"
    );
    // Eight characters, not eight bytes: seven two-byte letters are too short.
    assert!(rules::prepare(v3(Some("ääääääää"), Some("eight888")), Stored::Nothing).is_ok());
    assert_eq!(
        field(rules::prepare(
            v3(Some("äääääää"), Some(PRIVACY)),
            Stored::Nothing
        )),
        "authPassword"
    );
}

fn refused_field(edit: impl FnOnce(&mut SnmpConfig)) -> &'static str {
    let mut config = config(SnmpVersion::V3, Some("monitor"));
    edit(&mut config);
    field(rules::prepare(
        input(config, None, Some(AUTH), Some(PRIVACY)),
        Stored::Nothing,
    ))
}

#[test]
fn d_5_configuration_fields_are_checked_and_named_as_the_client_names_them() {
    assert_eq!(refused_field(|c| c.port = 0), "port");
    assert_eq!(refused_field(|c| c.timeout_ms = 99), "timeoutMs");
    assert_eq!(
        refused_field(|c| c.expected = Some("x".repeat(1025))),
        "expected"
    );
    assert_eq!(refused_field(|c| c.username = None), "username");
    assert_eq!(
        refused_field(|c| c.username = Some(String::new())),
        "username"
    );
    assert_eq!(
        refused_field(|c| c.username = Some("u".repeat(33))),
        "username"
    );
    assert_eq!(
        refused_field(|c| c.username = Some("a\nb".to_owned())),
        "username"
    );
    assert_eq!(refused_field(|c| c.version = SnmpVersion::V2c), "username");
    for oid in [
        "",
        "1",
        "3.1",
        "1.40",
        "1..3",
        ".1.3",
        "1.3.",
        "1.3.4294967296",
        "1.3.x",
        "1.3.+6",
    ] {
        assert_eq!(refused_field(|c| c.oid = oid.to_owned()), "oid", "{oid}");
    }
    let mut accepted = config(SnmpVersion::V3, Some(&"u".repeat(32)));
    accepted.expected = Some("x".repeat(1024));
    for oid in [
        "1.3",
        "0.39",
        "2.40",
        "2.4294967295.1",
        "1.3.6.1.4.1.4294967295",
    ] {
        accepted.oid = oid.to_owned();
        let prepared = rules::prepare(
            input(accepted.clone(), None, Some(AUTH), Some(PRIVACY)),
            Stored::Nothing,
        );
        assert!(prepared.is_ok(), "{oid}");
    }
    let too_many = vec!["1"; 129].join(".");
    assert_eq!(refused_field(|c| c.oid = too_many), "oid");
}
