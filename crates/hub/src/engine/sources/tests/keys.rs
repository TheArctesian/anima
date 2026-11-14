//! PRJ-018: the App key is checked before anything is sent, signs a short RS256 token that its
//! public key verifies, and opens from its stored form only for its own installation.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{TimeZone as _, Utc};
use ring::{
    rand::SystemRandom,
    signature::{RSA_PKCS1_2048_8192_SHA256, UnparsedPublicKey},
};
use serde_json::Value;

use super::{TEST_KEY, pem_of_bits};
use crate::engine::sources::acquisition::{
    app_key::{AppKey, KeyRefusal, MAX_PEM_BYTES},
    stored_key::{StoredKeyError, from_secret, to_secret},
};

#[test]
fn prj_018_each_bad_part_of_a_key_is_refused_by_its_field() {
    let pem = TEST_KEY.as_bytes();
    let too_long = vec![b'a'; MAX_PEM_BYTES + 1];
    let short = pem_of_bits(1024);
    let cases: [(&str, &str, &[u8], KeyRefusal); 9] = [
        ("", "2", pem, KeyRefusal::AppId),
        ("12a", "2", pem, KeyRefusal::AppId),
        ("0", "2", pem, KeyRefusal::AppId),
        ("123456789012345678901", "2", pem, KeyRefusal::AppId),
        ("1", "", pem, KeyRefusal::InstallationId),
        ("1", "-2", pem, KeyRefusal::InstallationId),
        ("1", "2", &too_long, KeyRefusal::TooLong),
        ("1", "2", b"not a key", KeyRefusal::NotRsaPem),
        ("1", "2", short.as_bytes(), KeyRefusal::TooShort),
    ];
    for (app, installation, key, want) in cases {
        let refusal = AppKey::read(app, installation, key).err();
        assert_eq!(refusal, Some(want), "{app:?} {installation:?}");
    }
    assert_eq!(KeyRefusal::TooShort.field(), "privateKey");
    assert!(!KeyRefusal::NotRsaPem.to_string().contains("BEGIN"));
}

#[test]
fn prj_018_the_app_token_is_rs256_with_the_app_id_and_a_ten_minute_window() {
    let key = AppKey::read("4242", "77", TEST_KEY.as_bytes()).unwrap();
    assert_eq!(key.instance_key(), "4242/77");
    let now = Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap();
    let token = key.token(now, &SystemRandom::new()).unwrap();
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
    let header: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
    assert_eq!(header["alg"], "RS256");
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
    assert_eq!(claims["iss"], 4242);
    assert_eq!(claims["iat"], now.timestamp() - 60);
    assert_eq!(claims["exp"], now.timestamp() + 540);
    let signed = format!("{}.{}", parts[0], parts[1]);
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    UnparsedPublicKey::new(&RSA_PKCS1_2048_8192_SHA256, key.public_key())
        .verify(signed.as_bytes(), &signature)
        .expect("the key's own public key verifies the token");
}

#[test]
fn prj_018_a_stored_key_opens_only_for_its_own_installation() {
    let secret = to_secret("4242", "77", &TEST_KEY);
    assert!(!String::from_utf8_lossy(secret.expose()).is_empty());
    assert!(from_secret(&secret, "4242/77").is_ok());
    assert_eq!(
        from_secret(&secret, "4242/78").err(),
        Some(StoredKeyError::OtherInstallation)
    );
    let garbage = crate::fleet::credentials::Secret::new(b"{\"v\":2}".to_vec());
    assert_eq!(
        from_secret(&garbage, "4242/77").err(),
        Some(StoredKeyError::Layout)
    );
}
