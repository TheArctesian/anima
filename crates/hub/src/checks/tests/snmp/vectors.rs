//! Known-answer vectors for the v3 cryptography (RFC 3414 A.3, RFC 7860, RFC 3826, RFC 4231,
//! NIST SP 800-38A).
//!
//! Sources of the expected values:
//! - SHA-256 password to key and localisation: no RFC lists them, so they were computed with
//!   Python 3 `hashlib` (`sha256` over "maplesyrup" repeated to 1 MiB, then
//!   `sha256(Ku + engineID + Ku)`), the method of RFC 3414 A.2.2 with the hash of RFC 7860.
//!   The same script gives the SHA-1 values printed in RFC 3414 A.3.2, which checks it.
//! - HMAC: RFC 4231 test case 2, truncated to its first 24 bytes (RFC 7860 Section 4.2.4).
//! - AES-128-CFB: NIST SP 800-38A F.3.13 (CFB128-AES128.Encrypt), also reproduced with
//!   `openssl enc -aes-128-cfb`.

use sha2::Digest as _;

use crate::checks::probe::snmp::usm::{self, Keys};

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

fn key(text: &str) -> usm::Key {
    let mut key = zeroize::Zeroizing::new([0_u8; 32]);
    key.copy_from_slice(&hex(text));
    key
}

const ENGINE: &str = "000000000000000000000002";
const KU: &str = "ab51014d1e077f6017df2b12bee5f5aa72993177e9bb569c4dff5a4ca0b4afac";
const KUL: &str = "8982e0e549e866db361a6b625d84cccc11162d453ee8ce3a6445c2d6776f0f8b";

#[test]
fn d_5_sha_256_password_to_key_matches_the_rfc_3414_a_3_method() {
    let ku = usm::password_key(b"maplesyrup").unwrap();
    assert_eq!(ku.as_slice(), hex(KU).as_slice());
}

#[test]
fn d_5_sha_256_key_localisation_matches_the_rfc_3414_a_3_method() {
    let local = usm::localise(&key(KU), &hex(ENGINE));
    assert_eq!(local.as_slice(), hex(KUL).as_slice());
    let both = Keys::localised(b"maplesyrup", b"maplesyrup", &hex(ENGINE)).unwrap();
    assert_eq!(both.auth.as_slice(), hex(KUL).as_slice());
    assert_eq!(both.privacy.as_slice(), hex(KUL).as_slice());
}

#[test]
fn d_5_an_empty_password_has_no_key() {
    assert!(usm::password_key(b"").is_none());
    assert!(Keys::localised(b"", b"maplesyrup", &hex(ENGINE)).is_none());
    assert!(Keys::localised(b"maplesyrup", b"", &hex(ENGINE)).is_none());
}

#[test]
fn d_5_a_password_longer_than_one_chunk_wraps_like_the_rfc_loop() {
    // 70 bytes do not divide 64: the repetition must carry across chunk ends.
    let password = [b'x'; 69].iter().chain(b"y").copied().collect::<Vec<u8>>();
    let ku = usm::password_key(&password).unwrap();
    let expanded: Vec<u8> = password.iter().copied().cycle().take(1024 * 1024).collect();
    assert_eq!(ku.as_slice(), sha2::Sha256::digest(&expanded).as_slice());
}

#[test]
fn d_5_hmac_sha_256_192_is_the_first_24_bytes_of_rfc_4231_case_2() {
    // The key is padded to 32 bytes by HMAC itself; "Jefe" is RFC 4231's key.
    let full = hex("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    let mut jefe = zeroize::Zeroizing::new([0_u8; 32]);
    jefe[..4].copy_from_slice(b"Jefe");
    // A zero-padded key gives the same HMAC as the short key (RFC 2104 Section 2).
    let digest = usm::digest(&jefe, b"what do ya want for nothing?");
    assert_eq!(digest.as_slice(), &full[..24]);
    assert!(usm::verify(&jefe, b"what do ya want for nothing?", &full[..24]));
    assert!(!usm::verify(&jefe, b"what do ya want for nothing!", &full[..24]));
    assert!(!usm::verify(&jefe, b"what do ya want for nothing?", &full[..23]));
    assert!(!usm::verify(&jefe, b"what do ya want for nothing?", &full));
}

const CFB_KEY: &str = "2b7e151628aed2a6abf7158809cf4f3c";
const CFB_PLAIN: &str = "6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51\
                         30c81c46a35ce411e5fbc1191a0a52eff69f2445df4f9b17ad2b417be66c3710";
const CFB_CIPHER: &str = "3b3fd92eb72dad20333449f8e83cfb4ac8a64537a0b3a93fcde3cdad9f1ce58b\
                          26751f67a3cbb140b1808cf187a4f4dfc04b05357c5d1c0eeac4c66f9ff7f2e6";

/// The NIST key in the first 16 bytes of a localised key; the rest must not be used.
fn cfb_key() -> usm::Key {
    let mut key = zeroize::Zeroizing::new([0xee_u8; 32]);
    key[..16].copy_from_slice(&hex(CFB_KEY));
    key
}

fn nist_iv() -> [u8; 16] {
    let mut iv = [0_u8; 16];
    iv.copy_from_slice(&hex("000102030405060708090a0b0c0d0e0f"));
    iv
}

#[test]
fn d_5_aes_128_cfb_matches_nist_sp_800_38a_f_3_13() {
    let mut data = hex(CFB_PLAIN);
    usm::encrypt(&cfb_key(), nist_iv(), &mut data);
    assert_eq!(data, hex(CFB_CIPHER));
    usm::decrypt(&cfb_key(), nist_iv(), &mut data);
    assert_eq!(data, hex(CFB_PLAIN));
}

#[test]
fn d_5_aes_128_cfb_handles_a_short_last_block() {
    // CFB is a stream mode: a prefix of the plaintext gives a prefix of the ciphertext.
    let mut data = hex(CFB_PLAIN)[..37].to_vec();
    usm::encrypt(&cfb_key(), nist_iv(), &mut data);
    assert_eq!(data, hex(CFB_CIPHER)[..37]);
    usm::decrypt(&cfb_key(), nist_iv(), &mut data);
    assert_eq!(data, hex(CFB_PLAIN)[..37]);
}

#[test]
fn d_5_the_iv_is_boots_then_time_then_salt() {
    let iv = usm::iv(0x0102_0304, 0x0506_0708, [9, 10, 11, 12, 13, 14, 15, 16]);
    assert_eq!(iv, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
}
