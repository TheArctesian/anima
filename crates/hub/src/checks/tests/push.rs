//! The push token: its text and its draw from the OS random source (CHK-002, CQ-021).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::checks::{PushToken, fresh_push_token};

#[test]
fn chk_002_token_text_is_unpadded_base64url_of_the_random_bytes() {
    let bytes: [u8; 32] = std::array::from_fn(|at| u8::try_from(at * 8 + 3).unwrap());
    let token = PushToken::from_random(&bytes);
    assert_eq!(token.expose().len(), 43);
    assert_eq!(URL_SAFE_NO_PAD.decode(token.expose()).unwrap(), bytes);
}

#[test]
fn chk_002_fresh_tokens_are_url_safe_and_differ() {
    let (first, second) = (fresh_push_token().unwrap(), fresh_push_token().unwrap());
    for token in [&first, &second] {
        assert_eq!(token.expose().len(), 43);
        assert!(
            token
                .expose()
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        );
    }
    assert_ne!(first.expose(), second.expose());
}
