//! What an API key request must be, and the key it becomes: a plain name of 1 to 64
//! characters, an expiry in the future and at most one year away, and a secret from 32 random
//! bytes (INT-003, D-23).
//!
//! Pure: the random bytes and the time are parameters.

use chrono::{DateTime, Months, Utc};

use super::Wanted;
use crate::{
    api_errors::FieldError,
    fleet::access::{
        Scope,
        tokens::{TOKEN_BYTES, Token},
    },
    text::is_plain,
};

/// Longest name, in characters (K2 0010-0029).
const NAME_MAX: usize = 64;

/// Longest life of a key from the moment it is made (D-23).
const LONGEST_LIFE: Months = Months::new(12);

/// A checked request and its token, ready to store. No `Debug`: it holds the token.
pub(super) struct Draft {
    pub name: String,
    pub scope: Scope,
    pub expires_at: DateTime<Utc>,
    pub token: Token,
}

/// The key `wanted` becomes at `now` with a secret made of `bytes`, or every field the
/// operator must fix.
pub(super) fn draft(
    wanted: &Wanted,
    bytes: [u8; TOKEN_BYTES],
    now: DateTime<Utc>,
) -> Result<Draft, Vec<FieldError>> {
    let token = Token::from_random(bytes);
    match (name(&wanted.name), expiry(wanted.expires_at, now)) {
        (Ok(name), Ok(expires_at)) => Ok(Draft {
            name,
            scope: wanted.scope,
            expires_at,
            token,
        }),
        (name, expires_at) => Err(name.err().into_iter().chain(expires_at.err()).collect()),
    }
}

/// The name without surrounding spaces: 1 to 64 characters, none of them a control or an
/// invisible character (Q2-11).
fn name(text: &str) -> Result<String, FieldError> {
    let name = text.trim();
    if !is_plain(name) {
        Err(FieldError::new(
            "name",
            "The name cannot contain control characters or invisible characters.",
        ))
    } else if (1..=NAME_MAX).contains(&name.chars().count()) {
        Ok(name.to_owned())
    } else {
        Err(FieldError::new(
            "name",
            format!("The name must be 1 to {NAME_MAX} characters."),
        ))
    }
}

/// The expiry when it is after `now` and at most one year after it.
fn expiry(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> Result<DateTime<Utc>, FieldError> {
    let latest = now
        .checked_add_months(LONGEST_LIFE)
        .unwrap_or(DateTime::<Utc>::MAX_UTC);
    if expires_at <= now {
        Err(FieldError::new(
            "expiresAt",
            "Choose an expiry date in the future.",
        ))
    } else if expires_at > latest {
        Err(FieldError::new(
            "expiresAt",
            "Choose an expiry date at most one year from now.",
        ))
    } else {
        Ok(expires_at)
    }
}
