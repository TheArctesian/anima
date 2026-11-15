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
