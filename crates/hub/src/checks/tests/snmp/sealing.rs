//! Sealed SNMP secrets: bound to their check by the vault context, and to the version, the
//! user name and the device host by the plaintext (SEC-003, SEC-005, SEC-010, D-5).

use std::io::Write as _;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use uuid::Uuid;

use crate::{
    checks::{
        Error,
        config::{SnmpConfig, SnmpSecrets, SnmpVersion},
        store::{open_snmp, seal_snmp},
    },
    credentials::{self, Sealed, Secret, Vault},
    ids::CheckId,
};

const COMMUNITY: &str = "c0mmunity-SECRET-sealed";
const AUTH: &str = "auth-SECRET-sealed";
const PRIVACY: &str = "privacy-SECRET-sealed";
/// The device host the secrets are entered for.
const HOST: &str = "nas-01.home.lan";

fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([9_u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

fn check() -> CheckId {
    CheckId::from(Uuid::now_v7())
}

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

fn user() -> SnmpSecrets {
    SnmpSecrets::User {
        auth: Secret::from(AUTH.to_owned()),
        privacy: Secret::from(PRIVACY.to_owned()),
    }
}

fn community() -> SnmpSecrets {
    SnmpSecrets::Community(Secret::from(COMMUNITY.to_owned()))
}

fn text(secret: &Secret) -> &str {
    std::str::from_utf8(secret.expose()).unwrap()
}

#[test]
fn sec_003_sealed_secrets_open_for_their_check_version_and_user() {
    let (vault, id) = (vault(), check());
    let v3 = config(SnmpVersion::V3, Some("monitor"));
    let sealed = seal_snmp(&vault, id, (&v3, HOST), &user()).unwrap();
    for secret in [AUTH, PRIVACY, "SECRET"] {
        assert!(!String::from_utf8_lossy(&sealed).contains(secret));
    }
    let Ok(SnmpSecrets::User { auth, privacy }) =
        open_snmp(&vault, id, (&v3, HOST), (&sealed, None))
    else {
        panic!("the v3 secrets did not open");
    };
    assert_eq!((text(&auth), text(&privacy)), (AUTH, PRIVACY));
    let v2c = config(SnmpVersion::V2c, None);
    let sealed = seal_snmp(&vault, id, (&v2c, HOST), &community()).unwrap();
    let Ok(SnmpSecrets::Community(opened)) = open_snmp(&vault, id, (&v2c, HOST), (&sealed, None))
    else {
        panic!("the community did not open");
    };
    assert_eq!(text(&opened), COMMUNITY);
}

#[test]
fn sec_003_the_plaintext_is_the_binding_length_the_binding_then_the_secrets() {
    let (vault, id) = (vault(), check());
    let v3 = config(SnmpVersion::V3, Some("monitor"));
    let bytes = seal_snmp(&vault, id, (&v3, HOST), &user()).unwrap();
    let context = format!("checks.checks.snmp:{id}");
    let plain = vault
        .open(&Sealed::from_bytes(&bytes).unwrap(), context.as_bytes())
        .unwrap();
    let binding = br#"{"version":"V3","username":"monitor","host":"nas-01.home.lan"}"#;
    let mut expected = u32::try_from(binding.len()).unwrap().to_be_bytes().to_vec();
    expected.extend_from_slice(binding);
    expected.push(2); // a v3 user
    for secret in [AUTH, PRIVACY] {
        expected.extend_from_slice(&u16::try_from(secret.len()).unwrap().to_be_bytes());
        expected.extend_from_slice(secret.as_bytes());
    }
    assert_eq!(plain.expose(), expected.as_slice());
}

#[test]
fn sec_003_secrets_sealed_for_one_check_do_not_open_for_another() {
    let vault = vault();
    let v2c = config(SnmpVersion::V2c, None);
    let sealed = seal_snmp(&vault, check(), (&v2c, HOST), &community()).unwrap();
    let opened = open_snmp(&vault, check(), (&v2c, HOST), (&sealed, None));
    assert!(matches!(
        opened,
        Err(Error::Credentials(credentials::Error::Decrypt))
    ));
}

#[test]
fn sec_005_another_version_or_user_name_gives_secrets_unbound() {
    let (vault, id) = (vault(), check());
    let v3 = config(SnmpVersion::V3, Some("monitor"));
    let sealed = seal_snmp(&vault, id, (&v3, HOST), &user()).unwrap();
    let others = [
        config(SnmpVersion::V3, Some("other")),
        config(SnmpVersion::V3, Some("Monitor")),
        config(SnmpVersion::V2c, None),
        config(SnmpVersion::V2c, Some("monitor")),
    ];
    for other in others {
        let opened = open_snmp(&vault, id, (&other, HOST), (&sealed, None));
        assert!(matches!(opened, Err(Error::SecretsUnbound)), "{other:?}");
    }
    // The port, OID and expected value are not part of the binding.
    let mut moved = v3.clone();
    moved.port = 1161;
    moved.oid = "1.3.6.1.2.1.1.3.0".to_owned();
    moved.expected = Some("x".to_owned());
    assert!(open_snmp(&vault, id, (&moved, HOST), (&sealed, None)).is_ok());
}

#[test]
fn sec_005_secrets_of_the_wrong_kind_for_the_binding_are_malformed() {
    let (vault, id) = (vault(), check());
    let v2c = config(SnmpVersion::V2c, None);
    let sealed = seal_snmp(&vault, id, (&v2c, HOST), &user()).unwrap();
    let opened = open_snmp(&vault, id, (&v2c, HOST), (&sealed, None));
    assert!(matches!(
        opened,
        Err(Error::Credentials(credentials::Error::Malformed))
    ));
}

#[test]
fn sec_003_garbage_and_http_layouts_do_not_open() {
    let (vault, id) = (vault(), check());
    let v2c = config(SnmpVersion::V2c, None);
    assert!(matches!(
        open_snmp(&vault, id, (&v2c, HOST), (b"not sealed", None)),
        Err(Error::Credentials(_))
    ));
    // A value sealed under the HTTP context of the same check does not open as SNMP.
    let http = vault
        .seal(
            &Secret::from(COMMUNITY.to_owned()),
            format!("checks.checks.http:{id}").as_bytes(),
        )
        .unwrap();
    assert!(matches!(
        open_snmp(&vault, id, (&v2c, HOST), (&http.to_bytes(), None)),
        Err(Error::Credentials(credentials::Error::Decrypt))
    ));
}

/// Each malformed plaintext under the right context is refused, never read past its end.
#[test]
fn sec_003_a_malformed_plaintext_is_refused() {
    let (vault, id) = (vault(), check());
    let v2c = config(SnmpVersion::V2c, None);
    let binding = br#"{"version":"V2C","username":null,"host":"nas-01.home.lan"}"#;
    let framed = |secrets: &[u8]| {
        let mut plain = u32::try_from(binding.len()).unwrap().to_be_bytes().to_vec();
        plain.extend_from_slice(binding);
        plain.extend_from_slice(secrets);
        plain
    };
    let cases: [Vec<u8>; 7] = [
        vec![0, 0],
        vec![0, 0, 1, 0],
        framed(&[]),
        framed(&[3, 0, 1, b'x']),
        framed(&[1, 0, 5, b'x']),
        framed(&[1, 0, 1, b'x', 0]),
        framed(&[2, 0, 1, b'x']),
    ];
    for plain in cases {
        let sealed = vault
            .seal(
                &Secret::new(plain.clone()),
                format!("checks.checks.snmp:{id}").as_bytes(),
            )
            .unwrap();
        let opened = open_snmp(&vault, id, (&v2c, HOST), (&sealed.to_bytes(), None));
        assert!(
            matches!(
                opened,
                Err(Error::Credentials(credentials::Error::Malformed))
            ),
            "{plain:?}"
        );
    }
    let sealed = vault
        .seal(
            &Secret::new(framed(&[1, 0, 1, b'x'])),
            format!("checks.checks.snmp:{id}").as_bytes(),
        )
        .unwrap();
    assert!(open_snmp(&vault, id, (&v2c, HOST), (&sealed.to_bytes(), None)).is_ok());
}

#[test]
fn sec_010_secrets_open_only_for_the_host_they_were_sealed_for() {
    let (vault, id) = (vault(), check());
    let v2c = config(SnmpVersion::V2c, None);
    let sealed = seal_snmp(&vault, id, (&v2c, HOST), &community()).unwrap();
    // The configured host text, without letter case; never an address it resolves to.
    for same in [HOST, "NAS-01.Home.LAN"] {
        assert!(
            open_snmp(&vault, id, (&v2c, same), (&sealed, None)).is_ok(),
            "{same}"
        );
    }
    for other in ["10.0.0.5", "nas-01.home.lan.example", "nas-02.home.lan", ""] {
        let opened = open_snmp(&vault, id, (&v2c, other), (&sealed, None));
        assert!(matches!(opened, Err(Error::SecretsForOtherHost)), "{other}");
        // A recorded legacy host never overrides the host in the binding.
        let opened = open_snmp(&vault, id, (&v2c, other), (&sealed, Some(other)));
        assert!(matches!(opened, Err(Error::SecretsForOtherHost)), "{other}");
    }
    // Another user name is unbound whatever the host.
    let v3 = config(SnmpVersion::V3, Some("monitor"));
    let sealed = seal_snmp(&vault, id, (&v3, HOST), &user()).unwrap();
    let other_user = config(SnmpVersion::V3, Some("other"));
    let opened = open_snmp(&vault, id, (&other_user, "10.0.0.5"), (&sealed, None));
    assert!(matches!(opened, Err(Error::SecretsUnbound)));
}

/// Secrets sealed before SEC-010 have no host in their binding: they open only for the host
/// migration 0324 recorded, and for none when it recorded nothing.
#[test]
fn sec_010_a_binding_without_a_host_opens_only_for_the_recorded_host() {
    let (vault, id) = (vault(), check());
    let v2c = config(SnmpVersion::V2c, None);
    let binding = br#"{"version":"V2C","username":null}"#;
    let mut plain = u32::try_from(binding.len()).unwrap().to_be_bytes().to_vec();
    plain.extend_from_slice(binding);
    plain.extend_from_slice(&[1, 0, 1, b'x']);
    let context = format!("checks.checks.snmp:{id}");
    let sealed = vault
        .seal(&Secret::new(plain), context.as_bytes())
        .unwrap()
        .to_bytes();
    let opened = open_snmp(&vault, id, (&v2c, HOST), (&sealed, None));
    assert!(matches!(opened, Err(Error::SecretsUnbound)));
    assert!(open_snmp(&vault, id, (&v2c, HOST), (&sealed, Some(HOST))).is_ok());
    let opened = open_snmp(&vault, id, (&v2c, "10.0.0.5"), (&sealed, Some(HOST)));
    assert!(matches!(opened, Err(Error::SecretsForOtherHost)));
}
