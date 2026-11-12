//! Property tests of the parsers: no text or byte string panics them, and a key shaped like a
//! real one but with random bytes is refused by its checksum (BLD-045, BLD-046).

use base64::{Engine, engine::general_purpose::STANDARD};
use proptest::prelude::*;

use super::tauri_key_file;
use crate::{
    PublicKey, SecretKey, SignatureCheck, SignatureError,
    key_file::{KEY_BYTES, OPS_AT, SALT_AT, SEALED_AT},
};

proptest! {
    #[test]
    fn bld_045_any_text_is_read_as_a_key_file_without_a_panic(text in ".{0,512}") {
        let _ = SecretKey::from_key_file(text.as_bytes(), None);
        let _ = SecretKey::from_key_file(STANDARD.encode(&text).as_bytes(), Some("pw"));
        let _ = PublicKey::from_key_file(&text);
        let _ = PublicKey::from_key_file(&STANDARD.encode(&text));
    }

    #[test]
    fn bld_045_random_key_bytes_never_open(
        sealed in proptest::collection::vec(any::<u8>(), KEY_BYTES - SEALED_AT),
        salt in proptest::collection::vec(any::<u8>(), OPS_AT - SALT_AT),
        encrypted in any::<bool>(),
    ) {
        let mut bytes = b"Ed\0\0B2".to_vec();
        if encrypted {
            bytes[2..4].copy_from_slice(b"Sc");
        }
        bytes.extend_from_slice(&salt);
        // The cheapest scrypt cost minisign reads: N = 2^10.
        bytes.extend_from_slice(&32_768_u64.to_le_bytes());
        bytes.extend_from_slice(&33_554_432_u64.to_le_bytes());
        prop_assert_eq!(bytes.len(), SEALED_AT);
        bytes.extend_from_slice(&sealed);
        let file = format!("untrusted comment: random\n{}\n", STANDARD.encode(&bytes));
        let opened = SecretKey::from_key_file(file.as_bytes(), Some("pw"));
        prop_assert!(opened.is_err());
    }

    #[test]
    fn bld_046_any_signature_text_is_read_without_a_panic(text in ".{0,512}") {
        let (file, _) = tauri_key_file(None);
        let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
        for given in [text.clone(), STANDARD.encode(&text)] {
            if let Ok(check) = SignatureCheck::new(&key.public_key(), &given) {
                let stream = check.stream();
                prop_assert!(stream.is_err() || stream.unwrap().finish("0.1.12").is_err());
            }
        }
    }
}

#[test]
fn bld_046_an_oversized_signature_text_is_refused_unread() {
    let (file, _) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    let huge = "A".repeat(8192);
    assert_eq!(
        SignatureCheck::new(&key.public_key(), &huge).err(),
        Some(SignatureError::Signature)
    );
}
