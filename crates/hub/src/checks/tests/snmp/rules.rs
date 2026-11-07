//! The SNMP input rules: field limits, the secrets of the chosen version, and when a secret
//! left out keeps the stored one (D-5, SEC-005, LOG-007).

use crate::{
    checks::{
        config::{SnmpConfig, SnmpCredentials, SnmpInput, SnmpSecrets, SnmpVersion},
        model::{Definition, FieldError},
        rules::{self, Prepared, Sealable, Stored},
    },
    credentials::Secret,
};

const COMMUNITY: &str = "c0mmunity-SECRET-rules";
const AUTH: &str = "auth-SECRET-rules";
const PRIVACY: &str = "privacy-SECRET-rules";
const KEPT_AUTH: &str = "kept-auth-SECRET";
const KEPT_PRIVACY: &str = "kept-privacy-SECRET";

fn config(version: SnmpVersion, username: Option<&str>) -> SnmpConfig {
    SnmpConfig {
        port: 161,
        version,
        oid: "1.3.6.1.2.1.1.5.0".to_owned(),
        expected: None,
        username: username.map(str::to_owned),
        timeout_ms: 5000,
    }
}

fn secret(text: &str) -> Secret {
    Secret::from(text.to_owned())
}

fn input(
    config: SnmpConfig,
    community: Option<&str>,
    auth: Option<&str>,
    privacy: Option<&str>,
) -> Definition {
    Definition::Snmp(SnmpInput {
        config,
        credentials: SnmpCredentials {
            community: community.map(secret),
            auth_password: auth.map(secret),
            privacy_password: privacy.map(secret),
        },
    })
}

fn v3(auth: Option<&str>, privacy: Option<&str>) -> Definition {
    input(
        config(SnmpVersion::V3, Some("monitor")),
        None,
        auth,
        privacy,
    )
}

fn stored_user() -> Stored {
    Stored::Snmp((
        config(SnmpVersion::V3, Some("monitor")),
        SnmpSecrets::User {
            auth: Secret::from(KEPT_AUTH.to_owned()),
            privacy: Secret::from(KEPT_PRIVACY.to_owned()),
        },
    ))
}

fn stored_community() -> Stored {
    Stored::Snmp((
        config(SnmpVersion::V2c, None),
        SnmpSecrets::Community(Secret::from("kept-community-SECRET".to_owned())),
    ))
}

fn text(secret: &Secret) -> &str {
    std::str::from_utf8(secret.expose()).unwrap()
}

fn passwords(prepared: Result<Prepared, FieldError>) -> (String, String) {
    let Some(Sealable::Snmp(SnmpSecrets::User { auth, privacy })) = prepared.unwrap().secrets
    else {
        panic!("no v3 secrets");
    };
    (text(&auth).to_owned(), text(&privacy).to_owned())
}

fn community(prepared: Result<Prepared, FieldError>) -> String {
    let Some(Sealable::Snmp(SnmpSecrets::Community(community))) = prepared.unwrap().secrets else {
        panic!("no community");
    };
    text(&community).to_owned()
}

/// The field in error; its message never quotes a planted secret.
fn field(prepared: Result<Prepared, FieldError>) -> &'static str {
    let Err(error) = prepared else {
        panic!("the input was accepted");
    };
    for planted in [
        COMMUNITY,
        AUTH,
        PRIVACY,
        KEPT_AUTH,
        KEPT_PRIVACY,
        "SECRET",
        "monitor",
    ] {
        assert!(!error.message.contains(planted), "{error}");
    }
    error.field
}

#[test]
fn d_5_create_needs_the_secrets_of_the_chosen_version() {
    let v2c = config(SnmpVersion::V2c, None);
    let created = rules::prepare(
        input(v2c.clone(), Some(COMMUNITY), None, None),
        Stored::Nothing,
    );
    assert_eq!(community(created), COMMUNITY);
    assert_eq!(
        field(rules::prepare(
            input(v2c.clone(), None, None, None),
            Stored::Nothing
        )),
        "community"
    );
    assert_eq!(
        field(rules::prepare(
            input(v2c, Some(""), None, None),
            Stored::Nothing
        )),
        "community"
    );
    let created = rules::prepare(v3(Some(AUTH), Some(PRIVACY)), Stored::Nothing);
    assert_eq!(passwords(created), (AUTH.to_owned(), PRIVACY.to_owned()));
    assert_eq!(
        field(rules::prepare(v3(None, Some(PRIVACY)), Stored::Nothing)),
        "authPassword"
    );
    assert_eq!(
        field(rules::prepare(v3(Some(AUTH), None), Stored::Nothing)),
        "privacyPassword"
    );
}

#[test]
fn sec_005_a_left_out_secret_keeps_the_stored_one_for_the_same_version_and_user() {
    let kept = rules::prepare(v3(None, None), stored_user());
    assert_eq!(
        passwords(kept),
        (KEPT_AUTH.to_owned(), KEPT_PRIVACY.to_owned())
    );
    let kept = rules::prepare(v3(Some(""), Some("")), stored_user());
    assert_eq!(
        passwords(kept),
        (KEPT_AUTH.to_owned(), KEPT_PRIVACY.to_owned())
    );
    let partly = rules::prepare(v3(Some(AUTH), None), stored_user());
    assert_eq!(
        passwords(partly),
        (AUTH.to_owned(), KEPT_PRIVACY.to_owned())
    );
    let partly = rules::prepare(v3(None, Some(PRIVACY)), stored_user());
    assert_eq!(
        passwords(partly),
        (KEPT_AUTH.to_owned(), PRIVACY.to_owned())
    );
    let v2c = config(SnmpVersion::V2c, None);
    let kept = rules::prepare(input(v2c, None, None, None), stored_community());
    assert_eq!(community(kept), "kept-community-SECRET");
}

#[test]
fn sec_005_a_changed_user_name_or_version_asks_for_the_secret() {
    let renamed = config(SnmpVersion::V3, Some("other"));
    let prepared = rules::prepare(
        input(renamed.clone(), None, None, Some(PRIVACY)),
        stored_user(),
    );
    assert_eq!(field(prepared), "authPassword");
    let prepared = rules::prepare(
        input(renamed.clone(), None, Some(AUTH), None),
        stored_user(),
    );
    assert_eq!(field(prepared), "privacyPassword");
    let given = rules::prepare(
        input(renamed, None, Some(AUTH), Some(PRIVACY)),
        stored_user(),
    );
    assert_eq!(passwords(given), (AUTH.to_owned(), PRIVACY.to_owned()));
    let to_v2c = input(config(SnmpVersion::V2c, None), None, None, None);
    assert_eq!(field(rules::prepare(to_v2c, stored_user())), "community");
    assert_eq!(
        field(rules::prepare(v3(None, Some(PRIVACY)), stored_community())),
        "authPassword"
    );
}

#[test]
fn d_5_only_a_left_out_secret_of_the_chosen_version_needs_the_stored_ones() {
    let needs = |definition: &Definition| match definition {
        Definition::Snmp(input) => rules::snmp_needs_stored(input),
        _ => unreachable!(),
    };
    assert!(needs(&v3(None, Some(PRIVACY))));
    assert!(needs(&v3(Some(AUTH), None)));
    assert!(needs(&v3(Some(AUTH), Some(""))));
    assert!(!needs(&v3(Some(AUTH), Some(PRIVACY))));
    let v2c = config(SnmpVersion::V2c, None);
    assert!(needs(&input(v2c.clone(), None, None, None)));
    assert!(needs(&input(v2c.clone(), Some(""), None, None)));
    assert!(!needs(&input(v2c, Some(COMMUNITY), None, None)));
}

#[test]
fn d_5_secrets_of_the_other_version_are_refused() {
    let v2c = config(SnmpVersion::V2c, None);
    let mixed = input(v2c.clone(), Some(COMMUNITY), Some(AUTH), None);
    assert_eq!(
        field(rules::prepare(mixed, Stored::Nothing)),
        "authPassword"
    );
    let mixed = input(v2c, Some(COMMUNITY), None, Some(PRIVACY));
    assert_eq!(
        field(rules::prepare(mixed, Stored::Nothing)),
        "privacyPassword"
    );
    let v3_config = config(SnmpVersion::V3, Some("monitor"));
    let mixed = input(v3_config, Some(COMMUNITY), Some(AUTH), Some(PRIVACY));
    assert_eq!(field(rules::prepare(mixed, Stored::Nothing)), "community");
}

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
