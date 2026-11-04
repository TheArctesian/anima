//! The pure rules: throttle, token text and hash, expiry, password check (D-7, D-8).

use std::fmt::{Debug, Display};

use axum::http::{HeaderMap, HeaderValue, header::AUTHORIZATION};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::TimeDelta;
use sha2::{Digest as _, Sha256};

use super::now;
use crate::access::{
    Password, identity,
    password::{self, DUMMY_HASH},
    throttle,
    tokens::{self, Token},
};

static_assertions::assert_not_impl_any!(Token: Debug, Display, Clone);
static_assertions::assert_not_impl_any!(Password: Debug, Display, Clone);

fn minutes(n: i64) -> TimeDelta {
    TimeDelta::minutes(n)
}

#[test]
fn sec_002_throttle_refuses_after_five_failures_within_15_minutes() {
    let start = now();
    let four: Vec<_> = (0..4).map(|i| start + minutes(i)).collect();
    assert!(!throttle::refused(&four, &[], start + minutes(4)));

    let five: Vec<_> = (0..5).map(|i| start + minutes(i)).collect();
    assert!(throttle::refused(&five, &[], start + minutes(4)));
    assert!(throttle::refused(&[], &five, start + minutes(4)), "by address");
}

#[test]
fn sec_002_throttle_releases_15_minutes_after_the_fifth_failure() {
    let start = now();
    let five: Vec<_> = (0..5).map(|i| start + minutes(i)).collect();
    let fifth = start + minutes(4);
    assert!(throttle::refused(&five, &[], fifth + minutes(15) - TimeDelta::seconds(1)));
    assert!(!throttle::refused(&five, &[], fifth + minutes(15)));
}

#[test]
fn sec_002_throttle_ignores_five_failures_spread_over_more_than_15_minutes() {
    let start = now();
    let spread: Vec<_> = (0..5).map(|i| start + minutes(4 * i)).collect();
    assert!(!throttle::refused(&spread, &[], start + minutes(16)));
}

#[test]
fn d_7_token_text_is_unpadded_base64url_of_the_random_bytes() {
    let bytes = [0xfb; tokens::TOKEN_BYTES];
    let token = Token::from_random(bytes);
    assert_eq!(token.expose(), URL_SAFE_NO_PAD.encode(bytes));
    assert_eq!(token.expose().len(), 43);
    let expected: [u8; 32] = Sha256::digest(bytes).into();
    assert!(token.hash() == tokens::TokenHash(expected));
}

#[test]
fn d_7_only_a_canonical_token_has_a_hash() {
    let token = Token::from_random([7; tokens::TOKEN_BYTES]);
    let text = token.expose();
    assert!(tokens::hash_of(text).is_some());
    assert!(tokens::hash_of(&format!("{text}=")).is_none(), "padding");
    assert!(tokens::hash_of(&text[..42]).is_none(), "short");
    assert!(tokens::hash_of(&"A".repeat(4096)).is_none(), "long");
    assert!(tokens::hash_of(&format!("{}+", &text[..42])).is_none(), "standard alphabet");
    // The last character carries two unused bits; a non-zero value is another spelling.
    let mut other = text[..42].to_owned();
    other.push('B');
    assert!(tokens::hash_of(&other).is_none(), "non-canonical trailing bits");
}

#[test]
fn d_7_token_is_usable_until_its_expiry_and_touched_once_a_minute() {
    let at = now();
    assert!(tokens::usable(at, at - TimeDelta::seconds(1)));
    assert!(!tokens::usable(at, at));
    assert!(tokens::touch_due(None, at));
    assert!(!tokens::touch_due(Some(at - TimeDelta::seconds(59)), at));
    assert!(tokens::touch_due(Some(at - TimeDelta::seconds(60)), at));
}

#[test]
fn api_003_bearer_header_is_read_once_case_insensitively() {
    let token = Token::from_random([9; tokens::TOKEN_BYTES]);
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("bearer {}", token.expose())).unwrap();
    headers.insert(AUTHORIZATION, value.clone());
    assert_eq!(identity::bearer(&headers), Some(token.expose()));

    headers.append(AUTHORIZATION, value);
    assert_eq!(identity::bearer(&headers), None, "two headers");

    let mut basic = HeaderMap::new();
    basic.insert(AUTHORIZATION, HeaderValue::from_static("Basic b3A6cHc="));
    assert_eq!(identity::bearer(&basic), None);
}

#[test]
fn d_8_dummy_hash_has_the_same_parameters_as_a_new_hash() {
    let fresh = password::hash(&Password::new("x".repeat(12)), &[1; password::SALT_LEN]).unwrap();
    let parameters = |phc: &str| phc.split('$').take(4).collect::<Vec<_>>().join("$");
    assert_eq!(parameters(DUMMY_HASH), parameters(&fresh));
    assert!(fresh.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
}

#[test]
fn sec_002_password_matches_only_its_hash() {
    let typed = "correct horse battery staple";
    let phc = password::hash(&Password::new(typed.to_owned()), &[2; password::SALT_LEN]).unwrap();
    assert!(password::matches(&Password::new(typed.to_owned()), &phc).unwrap());
    assert!(!password::matches(&Password::new("wrong".to_owned()), &phc).unwrap());
    assert!(!password::matches(&Password::new(typed.to_owned()), DUMMY_HASH).unwrap());
    let too_long = Password::new("x".repeat(password::MAX_PASSWORD_LEN + 1));
    assert!(!password::matches(&too_long, DUMMY_HASH).unwrap());
    assert!(password::matches(&Password::new(String::new()), "not a hash").is_err());
}
