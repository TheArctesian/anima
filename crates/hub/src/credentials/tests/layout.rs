use super::{CONTEXT, secret, test_key, vault_from};
use crate::credentials::{Error, Sealed};

#[test]
fn sec_003_sealed_bytes_round_trip() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), CONTEXT).unwrap();
    let parsed = Sealed::from_bytes(&sealed.to_bytes()).unwrap();
    assert_eq!(parsed, sealed);
    assert_eq!(vault.open(&parsed, CONTEXT).unwrap().expose(), b"s");
}

#[test]
fn sec_003_malformed_sealed_bytes_are_rejected() {
    let good = vault_from(test_key())
        .seal(&secret("s"), CONTEXT)
        .unwrap()
        .to_bytes();
    let mut bad_version = good.clone();
    bad_version[0] = 0x02;
    let mut long_id = good.clone();
    long_id[1] = 0xff;
    let mut not_utf8 = good.clone();
    not_utf8[2] = 0xff;
    let short_nonce = good[..2 + 16 + 23].to_vec();
    let short_ciphertext = good[..2 + 16 + 24 + 15].to_vec();
    let cases: [&[u8]; 8] = [
        &[],
        &[0x01],
        &[0x01, 0x00],
        &bad_version,
        &long_id,
        &not_utf8,
        &short_nonce,
        &short_ciphertext,
    ];
    for case in cases {
        assert!(matches!(Sealed::from_bytes(case), Err(Error::Malformed)));
    }
}

/// Property: every prefix and every single-byte change of a valid value either fails to
/// parse or fails to open; nothing panics and nothing opens to other plaintext.
#[test]
fn sec_003_any_altered_sealed_bytes_never_open() {
    let vault = vault_from(test_key());
    let good = vault.seal(&secret("property"), CONTEXT).unwrap().to_bytes();
    for len in 0..good.len() {
        match Sealed::from_bytes(&good[..len]) {
            Ok(sealed) => assert!(vault.open(&sealed, CONTEXT).is_err(), "prefix {len}"),
            Err(error) => assert!(matches!(error, Error::Malformed), "prefix {len}"),
        }
    }
    for index in 0..good.len() {
        for flip in [0x01_u8, 0x80, 0xff] {
            let mut bytes = good.clone();
            bytes[index] ^= flip;
            if let Ok(sealed) = Sealed::from_bytes(&bytes) {
                assert!(
                    vault.open(&sealed, CONTEXT).is_err(),
                    "byte {index} flip {flip:#x}"
                );
            }
        }
    }
}

#[test]
fn sec_003_from_parts_rejects_a_short_nonce() {
    let result = Sealed::from_parts(&[0; 23], vec![0; 16], "0123456789abcdef".to_owned());
    assert!(matches!(result, Err(Error::Malformed)));
}

#[test]
fn sec_003_from_parts_rejects_an_empty_or_overlong_key_id() {
    for key_id in [String::new(), "a".repeat(256)] {
        let result = Sealed::from_parts(&[0; 24], vec![0; 16], key_id);
        assert!(matches!(result, Err(Error::Malformed)));
    }
}

#[test]
fn sec_003_longest_key_id_round_trips_through_bytes() {
    let sealed = Sealed::from_parts(&[7; 24], vec![9; 16], "a".repeat(255)).unwrap();
    assert_eq!(Sealed::from_bytes(&sealed.to_bytes()).unwrap(), sealed);
}
