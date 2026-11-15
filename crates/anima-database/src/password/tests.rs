use static_assertions::assert_not_impl_any;

use super::Password;

assert_not_impl_any!(Password: std::fmt::Debug, std::fmt::Display, Clone);

#[test]
fn log_007_a_password_is_48_lower_case_hexadecimal_characters() {
    let password = Password::from_bytes(&[0xab; 24]);

    assert_eq!(password.expose(), "ab".repeat(24));
}

#[test]
fn each_byte_keeps_its_leading_zero() {
    let mut bytes = [0_u8; 24];
    bytes[0] = 0x0f;
    bytes[23] = 0xf0;

    let password = Password::from_bytes(&bytes);

    assert_eq!(password.expose().len(), 48);
    assert!(password.expose().starts_with("0f00"));
    assert!(password.expose().ends_with("00f0"));
}

#[test]
fn two_random_passwords_differ() {
    let first = Password::random().unwrap();
    let second = Password::random().unwrap();

    assert_ne!(first.expose(), second.expose());
}

/// The example of RFC 7677, Section 3: user `user`, password `pencil`.
#[test]
fn the_verifier_is_the_one_of_the_scram_sha_256_standard() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let salt: [u8; 16] = STANDARD
        .decode("W22ZaJ0SNY7soEsUEjb6gQ==")
        .unwrap()
        .try_into()
        .unwrap();

    let verifier = Password::from_text("pencil".to_owned()).scram_verifier(&salt);

    assert_eq!(
        verifier,
        "SCRAM-SHA-256$4096:W22ZaJ0SNY7soEsUEjb6gQ==\
         $WG5d8oPm3OtcPnkdi4Uo7BkeZkBFzpcXkuLmtbsT4qY=:wfPLwcE6nTWhTAmQ7tl2KeoiWGPlZqQxSrmfPwDl2dU="
    );
}

#[test]
fn log_007_the_verifier_does_not_hold_the_password() {
    let password = Password::from_bytes(&[0xab; 24]);

    let verifier = password.scram_verifier(&[1; 16]);

    assert!(!verifier.contains(password.expose()));
    assert!(
        !verifier.contains('\''),
        "it goes into a statement unquoted"
    );
}
