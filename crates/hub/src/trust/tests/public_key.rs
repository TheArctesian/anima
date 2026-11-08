//! Key blobs parse with every length checked; fingerprints are OpenSSH's (SEC-006, SEC-007).

use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::{ECDSA, ECDSA_FINGERPRINT, ED25519, ED25519_FINGERPRINT, Inputs};
use crate::trust::{
    BadFingerprint, BadKey, Fingerprint, PublicKey,
    public_key::{MAX_BASE64, MAX_BLOB},
};

fn blob(algorithm: &[u8], data: &[u8]) -> Vec<u8> {
    let mut blob = u32::try_from(algorithm.len())
        .unwrap()
        .to_be_bytes()
        .to_vec();
    blob.extend_from_slice(algorithm);
    blob.extend_from_slice(data);
    blob
}

#[test]
fn sec_006_fingerprint_is_the_one_ssh_keygen_prints() {
    for (text, algorithm, fingerprint) in [
        (ED25519, "ssh-ed25519", ED25519_FINGERPRINT),
        (ECDSA, "ecdsa-sha2-nistp256", ECDSA_FINGERPRINT),
    ] {
        let key = PublicKey::from_base64(text).unwrap();
        assert_eq!(key.algorithm(), algorithm);
        assert_eq!(key.fingerprint().as_str(), fingerprint);
        assert_eq!(Fingerprint::of_key(key.blob()).to_string(), fingerprint);
        assert_eq!(key.to_base64(), text, "base64 round-trips with padding");
    }
}

#[test]
fn sec_007_a_blob_cut_inside_its_algorithm_name_is_refused() {
    let full = STANDARD.decode(ED25519).unwrap();
    // 4 length bytes and the 11 bytes of `ssh-ed25519`.
    for cut in 0..15 {
        assert_eq!(
            PublicKey::from_blob(full[..cut].to_vec()),
            Err(BadKey::Truncated),
            "cut at {cut}"
        );
    }
    assert_eq!(
        PublicKey::from_blob(full[..15].to_vec()),
        Err(BadKey::NoKeyData)
    );
}

#[test]
fn sec_007_a_length_past_the_blob_is_refused_without_reading_it() {
    let mut huge = u32::MAX.to_be_bytes().to_vec();
    huge.extend_from_slice(b"ssh-ed25519 and more");
    assert_eq!(PublicKey::from_blob(huge), Err(BadKey::Truncated));
}

#[test]
fn sec_007_an_algorithm_name_that_could_break_a_known_hosts_line_is_refused() {
    let long = vec![b'a'; 65];
    let names: [&[u8]; 6] = [
        b"",
        b"ssh ed25519",
        b"ssh-ed25519\n",
        b"ssh\0rsa",
        "ssh-éd".as_bytes(),
        &long,
    ];
    for name in names {
        assert_eq!(
            PublicKey::from_blob(blob(name, b"key")),
            Err(BadKey::BadAlgorithm),
            "{name:?}"
        );
    }
    let key = PublicKey::from_blob(blob(b"sk-ssh-ed25519@openssh.com", b"k")).unwrap();
    assert_eq!(key.algorithm(), "sk-ssh-ed25519@openssh.com");
}

#[test]
fn sec_007_the_size_cap_is_8190_bytes_or_10920_base64_characters() {
    assert_eq!((MAX_BLOB, MAX_BASE64), (8190, 10_920));
    let largest = blob(b"ssh-rsa", &vec![1; MAX_BLOB - 11]);
    assert_eq!(largest.len(), MAX_BLOB);
    let text = STANDARD.encode(&largest);
    assert_eq!(text.len(), MAX_BASE64);
    assert!(PublicKey::from_base64(&text).is_ok());
    let over = blob(b"ssh-rsa", &vec![1; MAX_BLOB - 10]);
    assert_eq!(PublicKey::from_blob(over.clone()), Err(BadKey::TooLong));
    assert_eq!(
        PublicKey::from_base64(&STANDARD.encode(over)),
        Err(BadKey::TooLong)
    );
}

#[test]
fn sec_007_an_oversized_key_or_bad_base64_is_refused() {
    let big = blob(b"ssh-rsa", &vec![1; 9 * 1024]);
    assert_eq!(PublicKey::from_blob(big.clone()), Err(BadKey::TooLong));
    assert_eq!(
        PublicKey::from_base64(&STANDARD.encode(big)),
        Err(BadKey::TooLong)
    );
    for text in [
        "",
        "AAAA!AAA",
        "AAAAC3Nza C1lZDI1NTE5",
        "AAAAB===",
        "AAAAC3NzaC1lZDI1NTE5\n",
    ] {
        assert!(PublicKey::from_base64(text).is_err(), "{text:?}");
    }
    // Non-canonical: the final character carries bits that padding drops.
    assert_eq!(PublicKey::from_base64("AAAAAQB="), Err(BadKey::NotBase64));
}

#[test]
fn sec_007_the_parser_never_panics_and_only_accepts_clean_names() {
    let mut inputs = Inputs(0x5eed_0006);
    for _ in 0..20_000 {
        let mut bytes = inputs.bytes(48);
        // Bias the length prefix towards small values so names are often in range.
        if bytes.len() >= 4 && inputs.below(2) == 0 {
            bytes[..3].fill(0);
        }
        if let Ok(key) = PublicKey::from_blob(bytes.clone()) {
            let name = key.algorithm();
            assert!(!name.is_empty() && name.len() <= 64);
            assert!(!name.contains([' ', '\n', '\r', '\t']));
            assert_eq!(key.blob(), bytes.as_slice());
        }
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/= \n!";
        let _ = PublicKey::from_base64(&inputs.text(alphabet, 80));
        let _ = inputs.text(alphabet, 60).parse::<Fingerprint>();
    }
}

#[test]
fn sec_006_fingerprint_text_must_be_exactly_openssh_shape() {
    assert_eq!(
        ED25519_FINGERPRINT.parse::<Fingerprint>().unwrap().as_str(),
        ED25519_FINGERPRINT
    );
    let digest = &ED25519_FINGERPRINT["SHA256:".len()..];
    let refused = [
        digest.to_owned(),
        format!("sha256:{digest}"),
        format!("SHA256:{digest}="),
        format!("SHA256:{}", &digest[1..]),
        format!("SHA256:{digest}A"),
        format!("SHA256:{}!", &digest[1..]),
        format!("MD5:{digest}"),
        // `B` (index 1) leaves trailing bits set: not canonical base64.
        format!("SHA256:{}B", &digest[..42]),
    ];
    for text in refused {
        assert_eq!(text.parse::<Fingerprint>(), Err(BadFingerprint), "{text}");
    }
}

#[test]
fn log_007_a_public_key_debug_shows_algorithm_and_fingerprint_only() {
    let key = PublicKey::from_base64(ED25519).unwrap();
    let shown = format!("{key:?}");
    assert!(shown.contains("ssh-ed25519") && shown.contains(ED25519_FINGERPRINT));
    assert!(!shown.contains(ED25519));
}
