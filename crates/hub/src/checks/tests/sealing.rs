//! Sealed HTTP secrets (SEC-003, SEC-005, Q2-14): the frame and the context values sealed
//! before must keep, the binding each open checks, and a seal that has nothing to bind to.
//! Through the database they are tested in `tests/it/checks/face/secrets.rs`.

use std::io::Write as _;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use uuid::Uuid;

use crate::{
    checks::{
        Error, Sealer,
        config::{CheckConfig, HttpAuth, HttpConfig, HttpHeader, HttpSecrets},
        rules::{Prepared, Sealable},
        sealing::sealed,
        store::{open_http, seal_http},
    },
    credentials::{self, Sealed, Secret, Vault},
    ids::CheckId,
};

const PASSWORD: &str = "pa55-SECRET-sealed";
const COOKIE: &str = "session=SECRET-sealed";

fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

fn check() -> CheckId {
    CheckId::from(Uuid::now_v7())
}

/// A check of PVE-01's web interface with basic authentication, a secret `Cookie` header
/// (written in upper case) and a plain `Accept` header.
fn config() -> HttpConfig {
    let mut config = HttpConfig::new("https://pve-01.home.lan:8006/api2/json?x=1");
    config.headers = vec![
        HttpHeader {
            name: "Accept".to_owned(),
            value: Some("application/json".to_owned()),
        },
        HttpHeader {
            name: "COOKIE".to_owned(),
            value: None,
        },
    ];
    config.auth = HttpAuth::Basic {
        username: "monitor".to_owned(),
    };
    config
}

fn secrets() -> HttpSecrets {
    HttpSecrets {
        password: Some(Secret::from(PASSWORD.to_owned())),
        bearer_token: None,
        headers: vec![Secret::from(COOKIE.to_owned())],
    }
}

/// The binding `config()` seals, as JSON.
const BINDING: &[u8] = br#"{"origin":"https://pve-01.home.lan:8006","headers":["cookie"],"auth":{"kind":"BASIC","username":"monitor"},"verify_tls":true}"#;

/// A plaintext in the sealed layout: the binding's length, the binding, the secrets.
fn framed(binding: &[u8], secrets: &[u8]) -> Vec<u8> {
    let mut plain = u32::try_from(binding.len()).unwrap().to_be_bytes().to_vec();
    plain.extend_from_slice(binding);
    plain.extend_from_slice(secrets);
    plain
}

/// The plaintext sealed under the HTTP context of `id`, as an earlier hub stored it.
fn sealed_before(vault: &Vault, id: CheckId, plain: Vec<u8>) -> Vec<u8> {
    let context = format!("checks.checks.http:{id}");
    let sealed = vault.seal(&Secret::new(plain), context.as_bytes()).unwrap();
    sealed.to_bytes()
}

fn opened_password(opened: Result<HttpSecrets, Error>) -> String {
    let secrets = opened.unwrap_or_else(|error| panic!("did not open: {error:?}"));
    String::from_utf8(secrets.password.unwrap().expose().to_vec()).unwrap()
}

#[test]
fn sec_003_the_plaintext_is_the_binding_length_the_binding_then_the_secrets() {
    let (vault, id) = (vault(), check());
    let bytes = seal_http(&vault, id, &config(), &secrets()).unwrap();
    for secret in [PASSWORD, COOKIE, "SECRET"] {
        assert!(!String::from_utf8_lossy(&bytes).contains(secret));
    }
    let context = format!("checks.checks.http:{id}");
    let plain = vault
        .open(&Sealed::from_bytes(&bytes).unwrap(), context.as_bytes())
        .unwrap();
    let expected = framed(BINDING, secrets().encode().expose());
    assert_eq!(plain.expose(), expected.as_slice());
}

#[test]
fn sec_003_a_value_sealed_before_still_opens_for_its_check_only() {
    let (vault, id) = (vault(), check());
    let plain = framed(BINDING, secrets().encode().expose());
    let bytes = sealed_before(&vault, id, plain);
    assert_eq!(
        opened_password(open_http(&vault, id, &config(), &bytes)),
        PASSWORD
    );
    assert!(matches!(
        open_http(&vault, check(), &config(), &bytes),
        Err(Error::Credentials(credentials::Error::Decrypt))
    ));
    let snmp = format!("checks.checks.snmp:{id}");
    let as_snmp = vault
        .seal(&Secret::new(framed(BINDING, b"{}")), snmp.as_bytes())
        .unwrap();
    assert!(matches!(
        open_http(&vault, id, &config(), &as_snmp.to_bytes()),
        Err(Error::Credentials(credentials::Error::Decrypt))
    ));
}

#[test]
fn sec_005_the_secrets_open_only_where_they_were_given_for() {
    let (vault, id) = (vault(), check());
    let bytes = seal_http(&vault, id, &config(), &secrets()).unwrap();
    let changed = |change: fn(&mut HttpConfig)| {
        let mut config = config();
        change(&mut config);
        config
    };
    let refused = [
        changed(|c| c.url = "https://attacker.example:8006/".to_owned()),
        changed(|c| c.url = "http://pve-01.home.lan:8006/".to_owned()),
        changed(|c| c.url = "https://pve-01.home.lan:8443/".to_owned()),
        changed(|c| c.url = "not a url".to_owned()),
        changed(|c| c.headers[1].name = "Authorization".to_owned()),
        changed(|c| c.headers[0].value = None),
        changed(|c| c.auth = HttpAuth::Bearer),
        changed(|c| {
            c.auth = HttpAuth::Basic {
                username: "attacker".to_owned(),
            };
        }),
        changed(|c| c.verify_tls = false),
    ];
    for config in refused {
        let opened = open_http(&vault, id, &config, &bytes);
        assert!(matches!(opened, Err(Error::SecretsUnbound)), "{config:?}");
    }
    let kept = [
        config(),
        changed(|c| c.url = "HTTPS://PVE-01.home.lan:8006/other?y=2".to_owned()),
        changed(|c| c.headers[1].name = "cookie".to_owned()),
        changed(|c| c.headers[0].value = Some("text/html".to_owned())),
    ];
    for config in kept {
        let opened = open_http(&vault, id, &config, &bytes);
        assert_eq!(opened_password(opened), PASSWORD, "{config:?}");
    }
}

#[test]
fn sec_005_secrets_given_without_tls_checks_open_once_they_are_turned_on() {
    let (vault, id) = (vault(), check());
    let mut unverified = config();
    unverified.verify_tls = false;
    let bytes = seal_http(&vault, id, &unverified, &secrets()).unwrap();
    assert_eq!(
        opened_password(open_http(&vault, id, &unverified, &bytes)),
        PASSWORD
    );
    assert_eq!(
        opened_password(open_http(&vault, id, &config(), &bytes)),
        PASSWORD
    );
}

#[test]
fn sec_005_a_binding_with_an_empty_origin_opens_for_nothing() {
    let (vault, id) = (vault(), check());
    let empty = br#"{"origin":"","headers":[],"auth":{"kind":"NONE"},"verify_tls":true}"#;
    let bytes = sealed_before(&vault, id, framed(empty, secrets().encode().expose()));
    for url in ["file:///etc/hosts", "not a url", "https://pve-01.home.lan/"] {
        let opened = open_http(&vault, id, &HttpConfig::new(url), &bytes);
        assert!(matches!(opened, Err(Error::SecretsUnbound)), "{url}");
    }
}

#[test]
fn q2_14_secrets_for_a_url_without_an_origin_are_not_sealed_and_name_the_url() {
    let (vault, id) = (vault(), check());
    for url in ["file:///etc/hosts", "mailto:ops@home.lan", "not a url"] {
        let mut config = config();
        config.url = url.to_owned();
        let prepared = Prepared {
            config: CheckConfig::Http(config),
            secrets: Some(Sealable::Http(secrets())),
            push_token: None,
        };
        let Err(Error::Validation(fields)) = sealed(
            Sealer {
                vault: &vault,
                host: "pve-01.home.lan",
            },
            id,
            &prepared,
        ) else {
            panic!("secrets for {url} were sealed");
        };
        assert_eq!(fields.len(), 1, "{url}");
        assert_eq!(fields[0].field, "url");
        assert_eq!(fields[0].message, "This value is not valid.");
    }
}
