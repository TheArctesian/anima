//! The pure API key rules: the secret's spelling from given bytes, and the name and expiry
//! bounds at a given time (INT-003, D-23). The stored and served behaviour is in the `it` crate
//! (`tests/it/api_keys/`).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Months, TimeDelta, TimeZone, Utc};
use sha2::{Digest as _, Sha256};

use super::{
    Wanted,
    rules::{Draft, draft},
};
use crate::fleet::access::{MadeFor, Scope, tokens::TokenHash};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 2, 28, 12, 0, 0).unwrap()
}

fn wanted(name: &str, expires_at: DateTime<Utc>) -> Wanted {
    Wanted {
        name: name.to_owned(),
        scope: Scope::Read,
        made_for: MadeFor::User,
        expires_at,
    }
}

fn drafted(name: &str, expires_at: DateTime<Utc>) -> Result<Draft, Vec<&'static str>> {
    draft(&wanted(name, expires_at), [5; 32], now())
        .map_err(|fields| fields.iter().map(|field| field.field).collect())
}

fn ok(name: &str, expires_at: DateTime<Utc>) -> Draft {
    drafted(name, expires_at).unwrap_or_else(|fields| panic!("{name:?}: {fields:?}"))
}

#[test]
fn int_003_d_23_the_secret_is_anima_and_the_base64url_of_the_random_bytes() {
    let key = ok("deploy", now() + TimeDelta::days(1));
    let secret = key.token.key_secret();
    let body = URL_SAFE_NO_PAD.encode([5_u8; 32]);
    assert_eq!(*secret, format!("anima_{body}"));
    assert_eq!(secret.len(), 49);
    assert_eq!(key.token.key_display_prefix(), secret[..12]);
    assert!(key.token.hash() == TokenHash(Sha256::digest([5_u8; 32]).into()));
}

#[test]
fn d_23_the_expiry_is_after_now_and_at_most_one_calendar_year_away() {
    let year = Utc.with_ymd_and_hms(2031, 2, 28, 12, 0, 0).unwrap();
    assert_eq!(now().checked_add_months(Months::new(12)), Some(year));
    for allowed in [now() + TimeDelta::microseconds(1), year] {
        assert_eq!(ok("k", allowed).expires_at, allowed);
    }
    for refused in [
        now(),
        now() - TimeDelta::days(1),
        year + TimeDelta::microseconds(1),
        DateTime::<Utc>::MAX_UTC,
    ] {
        assert_eq!(
            drafted("k", refused).err(),
            Some(vec!["expiresAt"]),
            "{refused}"
        );
    }
}

/// From 29 February the year runs to 28 February: chrono clamps the month's last day.
#[test]
fn d_23_a_key_made_on_29_february_lasts_until_28_february() {
    let leap = Utc.with_ymd_and_hms(2028, 2, 29, 12, 0, 0).unwrap();
    let year = Utc.with_ymd_and_hms(2029, 2, 28, 12, 0, 0).unwrap();
    let expiring = |at| draft(&wanted("k", at), [5; 32], leap).map(|key| key.expires_at);
    assert_eq!(expiring(year).ok(), Some(year));
    assert!(expiring(year + TimeDelta::microseconds(1)).is_err());
}

#[test]
fn d_23_the_name_is_trimmed_plain_and_1_to_64_characters() {
    let later = now() + TimeDelta::days(1);
    assert_eq!(ok("  nightly backup ", later).name, "nightly backup");
    assert_eq!(ok(&"é".repeat(64), later).name.chars().count(), 64);
    for refused in [
        String::new(),
        " \t ".to_owned(),
        "x".repeat(65),
        "zero\u{200b}width".to_owned(),
        "right\u{202e}left".to_owned(),
        "tag\u{e0041}".to_owned(),
        "bell\u{7}".to_owned(),
    ] {
        assert_eq!(
            drafted(&refused, later).err(),
            Some(vec!["name"]),
            "{refused:?}"
        );
    }
}

#[test]
fn d_23_every_field_to_fix_is_listed() {
    let fields = drafted("", now()).err();
    assert_eq!(fields, Some(vec!["name", "expiresAt"]));
}

#[test]
fn d_23_the_scope_is_kept() {
    let mut input = wanted("w", now() + TimeDelta::days(1));
    input.scope = Scope::Write;
    let key = draft(&input, [1; 32], now()).unwrap_or_else(|_| panic!("valid"));
    assert_eq!(key.scope, Scope::Write);
}
