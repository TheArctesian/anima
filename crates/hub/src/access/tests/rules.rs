//! The pure rules: throttle, token text and hash, expiry, password check (D-7, D-8).

use std::fmt::{Debug, Display};

use axum::http::{HeaderMap, HeaderValue, header::AUTHORIZATION};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::TimeDelta;
use sha2::{Digest as _, Sha256};

use super::now;
use crate::access::{
    Error, Password, Scope, check_credentials, identity, login,
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
    assert!(!throttle::refused(&four, start + minutes(4)));

    let five: Vec<_> = (0..5).map(|i| start + minutes(i)).collect();
    assert!(throttle::refused(&five, start + minutes(4)));
}

#[test]
fn sec_002_throttle_releases_15_minutes_after_the_fifth_failure() {
    let start = now();
    let five: Vec<_> = (0..5).map(|i| start + minutes(i)).collect();
    let fifth = start + minutes(4);
    assert!(throttle::refused(
        &five,
        fifth + minutes(15) - TimeDelta::seconds(1)
    ));
    assert!(!throttle::refused(&five, fifth + minutes(15)));
}

#[test]
fn sec_002_throttle_ignores_five_failures_spread_over_more_than_15_minutes() {
    let start = now();
    let spread: Vec<_> = (0..5).map(|i| start + minutes(4 * i)).collect();
    assert!(!throttle::refused(&spread, start + minutes(16)));
}

#[test]
fn sec_002_d_7_token_text_is_unpadded_base64url_of_the_random_bytes() {
    let bytes = [0xfb; tokens::TOKEN_BYTES];
    let token = Token::from_random(bytes);
    assert_eq!(token.expose(), URL_SAFE_NO_PAD.encode(bytes));
    assert_eq!(token.expose().len(), 43);
    let expected: [u8; 32] = Sha256::digest(bytes).into();
    assert!(token.hash() == tokens::TokenHash(expected));
}

#[test]
fn sec_002_d_7_only_a_canonical_token_has_a_hash() {
    let token = Token::from_random([7; tokens::TOKEN_BYTES]);
    let text = token.expose();
    assert!(tokens::hash_of(text).is_some());
    assert!(tokens::hash_of(&format!("{text}=")).is_none(), "padding");
    assert!(tokens::hash_of(&text[..42]).is_none(), "short");
    assert!(tokens::hash_of(&"A".repeat(4096)).is_none(), "long");
    assert!(
        tokens::hash_of(&format!("{}+", &text[..42])).is_none(),
        "standard alphabet"
    );
    // The last character carries two unused bits; a non-zero value is another spelling.
    let mut other = text[..42].to_owned();
    other.push('B');
    assert!(
        tokens::hash_of(&other).is_none(),
        "non-canonical trailing bits"
    );
}

#[test]
fn sec_002_d_7_token_is_usable_until_its_expiry_and_touched_once_a_minute() {
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
fn sec_002_d_8_dummy_hash_has_the_same_parameters_as_a_new_hash() {
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

#[test]
fn sec_002_d_8_throttle_window_includes_exactly_15_minutes() {
    let start = now();
    let mut edge: Vec<_> = (0..4).map(|i| start + minutes(i)).collect();
    edge.push(start + minutes(15));
    assert!(
        throttle::refused(&edge, start + minutes(15)),
        "span of exactly 15 minutes"
    );
    let mut over: Vec<_> = (0..4).map(|i| start + minutes(i)).collect();
    over.push(start + minutes(15) + TimeDelta::seconds(1));
    assert!(
        !throttle::refused(&over, start + minutes(16)),
        "span over 15 minutes"
    );
}

fn refused_field(username: &str, password: &str) -> Option<&'static str> {
    match check_credentials(username, &Password::new(password.to_owned())) {
        Ok(()) => None,
        Err(Error::Validation { field, .. }) => Some(field),
        Err(other) => panic!("unexpected {other}"),
    }
}

#[test]
fn sec_002_d_9_operator_set_password_needs_12_characters_and_at_most_1024_bytes() {
    let name = "operator";
    assert_eq!(refused_field(name, &"p".repeat(11)), Some("password"));
    assert_eq!(refused_field(name, &"p".repeat(12)), None);
    assert_eq!(
        refused_field(name, &"\u{e9}".repeat(12)),
        None,
        "characters, not bytes"
    );
    assert_eq!(refused_field(name, &"p".repeat(1024)), None);
    assert_eq!(refused_field(name, &"p".repeat(1025)), Some("password"));
    assert_eq!(
        refused_field(name, &"\u{e9}".repeat(513)),
        Some("password"),
        "1026 bytes"
    );
}

#[test]
fn sec_002_d_9_operator_set_username_is_1_to_64_plain_characters() {
    let password = "correct horse battery staple";
    for good in ["a", "op.er_at-or9", &"a".repeat(64)] {
        assert_eq!(refused_field(good, password), None, "{good}");
    }
    for bad in [
        "",
        "an operator",
        "op\0",
        "op\u{e9}",
        "op/x",
        &"a".repeat(65),
    ] {
        assert_eq!(refused_field(bad, password), Some("username"), "{bad:?}");
    }
}

#[test]
fn sec_002_login_attempts_keep_at_most_128_characters_without_nul() {
    let long = "\u{e9}".repeat(200);
    assert_eq!(login::stored_username(&long), "\u{e9}".repeat(128));
    assert_eq!(login::stored_username("a\0b"), "a\u{fffd}b");
    assert_eq!(login::stored_username("operator"), "operator");
}

#[test]
fn sec_002_d_7_scope_round_trips_its_stored_text() {
    for scope in [Scope::Read, Scope::Write, Scope::Builder] {
        assert_eq!(Scope::from_stored(scope.as_str()), Some(scope));
    }
    assert_eq!(Scope::from_stored("read"), Some(Scope::Read));
    assert_eq!(Scope::from_stored("admin"), None);
}

#[test]
fn api_010_an_unreachable_database_is_unavailable_and_logged_by_kind_only() {
    let io = sqlx::Error::Io(std::io::Error::from(std::io::ErrorKind::ConnectionRefused));
    for source in [sqlx::Error::PoolTimedOut, sqlx::Error::PoolClosed, io] {
        let error = Error::from(source);
        assert_eq!(error.code(), "UNAVAILABLE");
        assert_eq!(error.fault(), Some("database_unreachable"));
    }
    let other = Error::from(sqlx::Error::RowNotFound);
    assert_eq!(
        (other.code(), other.fault()),
        ("INTERNAL", Some("database"))
    );
    assert_eq!(other.to_string(), "Something went wrong on the hub.");
    assert_eq!(Error::WrongCredentials.fault(), None);
}

#[test]
fn cq_023_a_totp_seal_fault_is_internal_not_a_wrong_password() {
    let random = Error::Random(getrandom::Error::UNSUPPORTED);
    assert_eq!(
        (random.code(), random.fault()),
        ("INTERNAL", Some("random_source"))
    );
    let seal = Error::TotpSeal(crate::credentials::Error::Malformed);
    assert_eq!((seal.code(), seal.fault()), ("INTERNAL", Some("totp_seal")));
    assert_eq!(seal.to_string(), "Something went wrong on the hub.");
}

#[test]
fn api_003_r_020_bearer_token_reads_one_case_insensitive_scheme() {
    assert_eq!(identity::bearer_token("Bearer abc"), Some("abc"));
    assert_eq!(identity::bearer_token("bEaReR  abc "), Some("abc"));
    assert_eq!(identity::bearer_token("Basic abc"), None);
    assert_eq!(identity::bearer_token("Bearerabc"), None);
    assert_eq!(identity::bearer_token("abc"), None);
}
