use super::{CONTEXT, other_vault, secret, test_key, vault_from};
use crate::fleet::credentials::{Error, Sealed, Secret};

fn with_ciphertext(sealed: &Sealed, ciphertext: Vec<u8>) -> Result<Sealed, Error> {
    Sealed::from_parts(sealed.nonce(), ciphertext, sealed.key_id().to_owned())
}

#[test]
fn sec_003_seal_then_open_returns_plaintext() {
    let vault = vault_from(test_key());
    let sealed = vault
        .seal(&secret("correct horse battery staple"), CONTEXT)
        .unwrap();
    assert_eq!(
        vault.open(&sealed, CONTEXT).unwrap().expose(),
        b"correct horse battery staple"
    );
}

#[test]
fn sec_003_empty_secret_round_trips() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&Secret::new(Vec::new()), CONTEXT).unwrap();
    assert_eq!(vault.open(&sealed, CONTEXT).unwrap().expose(), b"");
}

#[test]
fn sec_003_sealing_twice_uses_fresh_nonces_and_ciphertexts() {
    let vault = vault_from(test_key());
    let first = vault.seal(&secret("same"), CONTEXT).unwrap();
    let second = vault.seal(&secret("same"), CONTEXT).unwrap();
    assert_ne!(first.nonce(), second.nonce());
    assert_ne!(first.ciphertext(), second.ciphertext());
}

#[test]
fn sec_003_ciphertext_does_not_contain_plaintext() {
    let sealed = vault_from(test_key())
        .seal(&secret("plain-marker-text"), CONTEXT)
        .unwrap();
    let marker = b"plain-marker-text";
    assert!(
        !sealed
            .ciphertext()
            .windows(marker.len())
            .any(|w| w == marker)
    );
}

#[test]
fn sec_003_wrong_key_fails() {
    let sealed = vault_from(test_key()).seal(&secret("s"), CONTEXT).unwrap();
    assert!(matches!(
        other_vault().open(&sealed, CONTEXT),
        Err(Error::WrongKey)
    ));
}

#[test]
fn sec_003_same_key_id_with_a_different_key_fails_to_decrypt() {
    let sealed = vault_from(test_key()).seal(&secret("s"), CONTEXT).unwrap();
    let other = other_vault();
    let forged = Sealed::from_parts(
        sealed.nonce(),
        sealed.ciphertext().to_vec(),
        other
            .seal(&secret("x"), CONTEXT)
            .unwrap()
            .key_id()
            .to_owned(),
    )
    .unwrap();
    assert!(matches!(other.open(&forged, CONTEXT), Err(Error::Decrypt)));
}

#[test]
fn sec_003_changed_nonce_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), CONTEXT).unwrap();
    let mut nonce = sealed.nonce().to_vec();
    nonce[0] ^= 0x01;
    let changed = Sealed::from_parts(
        &nonce,
        sealed.ciphertext().to_vec(),
        sealed.key_id().to_owned(),
    )
    .unwrap();
    assert!(matches!(vault.open(&changed, CONTEXT), Err(Error::Decrypt)));
}

#[test]
fn sec_003_changed_ciphertext_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("secret text"), CONTEXT).unwrap();
    for bit in 0..sealed.ciphertext().len() * 8 {
        let mut ciphertext = sealed.ciphertext().to_vec();
        ciphertext[bit / 8] ^= 1 << (bit % 8);
        let changed = with_ciphertext(&sealed, ciphertext).unwrap();
        assert!(
            matches!(vault.open(&changed, CONTEXT), Err(Error::Decrypt)),
            "bit {bit}"
        );
    }
}

#[test]
fn sec_003_truncated_ciphertext_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("secret text"), CONTEXT).unwrap();
    for len in 0..sealed.ciphertext().len() {
        let truncated = sealed.ciphertext()[..len].to_vec();
        match with_ciphertext(&sealed, truncated) {
            Ok(changed) => assert!(matches!(vault.open(&changed, CONTEXT), Err(Error::Decrypt))),
            Err(error) => assert!(matches!(error, Error::Malformed)),
        }
    }
}

#[test]
fn sec_003_changed_key_id_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), CONTEXT).unwrap();
    let changed = Sealed::from_parts(
        sealed.nonce(),
        sealed.ciphertext().to_vec(),
        "0000000000000000".to_owned(),
    )
    .unwrap();
    assert!(matches!(
        vault.open(&changed, CONTEXT),
        Err(Error::WrongKey)
    ));
}

#[test]
fn sec_003_other_context_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), b"device-a/ssh_password").unwrap();
    let opened = vault.open(&sealed, b"device-a/ssh_password");
    assert_eq!(opened.unwrap().expose(), b"s");
    for context in [
        &b"device-b/ssh_password"[..],
        b"device-a/ssh_passphrase",
        b"",
    ] {
        assert!(matches!(vault.open(&sealed, context), Err(Error::Decrypt)));
    }
}

#[test]
fn sec_003_empty_context_is_not_a_prefix_of_another() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), b"").unwrap();
    assert_eq!(vault.open(&sealed, b"").unwrap().expose(), b"s");
    assert!(matches!(vault.open(&sealed, b"\0"), Err(Error::Decrypt)));
}

fn bytes_from_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
        .collect()
}

/// Known answer: a value sealed by the first release with the test key. It pins the
/// associated-data encoding (format tag, key id, context, length prefixes). If this value
/// must change, every secret already stored no longer opens.
#[test]
fn sec_003_known_sealed_value_still_opens() {
    let stored = bytes_from_hex(concat!(
        "011032343037393733626639663261333239826c7d6a1e9b21f907ccb2d9892a",
        "9038092a1be8ea4ddacd5a7f5a32caa874ce5ff1a2ff6c8ee9295514c2",
    ));
    let sealed = Sealed::from_bytes(&stored).unwrap();
    let opened = vault_from(test_key()).open(&sealed, b"credentials.secrets:dev:ssh_password");
    assert_eq!(opened.unwrap().expose(), b"kat");
}
