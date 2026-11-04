//! RFC 6238 time-based one-time codes: HMAC-SHA-1, 6 digits, 30-second steps, one step of
//! clock drift either way, and no step used twice (SEC-002, OD-2).
//!
//! Pure: the secret, the time and the last used step are parameters. HMAC comes from the
//! `RustCrypto` `hmac` and `sha1` crates; codes are compared with `subtle` in constant time.

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha1::Sha1;
use subtle::ConstantTimeEq as _;
use zeroize::Zeroizing;

use super::store::Account;
use crate::{
    credentials::{self, Sealed, Vault},
    ids::OperatorId,
};

/// Secret length in bytes: 160 bits, the HMAC-SHA-1 output size RFC 4226 recommends.
pub(super) const SECRET_LEN: usize = 20;

/// Digits in a code.
const DIGITS: usize = 6;

/// Seconds in one time step.
const STEP_SECONDS: i64 = 30;

/// Steps of clock drift accepted on either side of the current one.
const DRIFT: i64 = 1;

/// The issuer authenticator apps show beside the account.
const ISSUER: &str = "Anima";

/// The Vault context that binds an operator's sealed secret to its row (R-010, K3).
pub(super) fn context(operator: OperatorId) -> String {
    format!("access.operators.totp:{operator}")
}

/// The time step that holds `at`: whole 30-second periods since the Unix epoch.
pub(super) const fn step_at(at: DateTime<Utc>) -> i64 {
    at.timestamp().div_euclid(STEP_SECONDS)
}

/// What the second factor makes of a sign-in whose password matched.
pub(super) enum SecondFactor {
    /// The operator has not enrolled TOTP.
    NotEnrolled,
    /// The code is valid for this step, which the sign-in must still claim as used.
    Claim(i64),
    /// The code is missing, malformed, wrong or for a step already used.
    Refuse,
    /// The stored secret does not open with this vault and the operator's context: a hub
    /// fault, never a sign-in.
    Unopenable(credentials::Error),
}

impl SecondFactor {
    /// Whether the sign-in may go on.
    pub(super) const fn passes(&self) -> bool {
        matches!(self, Self::NotEnrolled | Self::Claim(_))
    }

    /// The step the sign-in must claim, if any.
    pub(super) const fn step(&self) -> Option<i64> {
        match self {
            Self::Claim(step) => Some(*step),
            _ => None,
        }
    }
}

/// Checks `code` against the operator's sealed TOTP secret at `at` (RFC 6238, R-010). A
/// missing or malformed code is refused before the secret is opened.
pub(super) fn second_factor(
    vault: &Vault,
    account: &Account,
    code: Option<&str>,
    at: DateTime<Utc>,
) -> SecondFactor {
    let Some(sealed) = account.totp_secret_enc.as_deref() else {
        return SecondFactor::NotEnrolled;
    };
    let Some(code) = code.filter(|code| is_code(code)) else {
        return SecondFactor::Refuse;
    };
    let context = context(account.id);
    let secret = match Sealed::from_bytes(sealed).and_then(|s| vault.open(&s, context.as_bytes())) {
        Ok(secret) => secret,
        Err(error) => return SecondFactor::Unopenable(error),
    };
    accepted_step(secret.expose(), code, at, account.totp_last_step)
        .map_or(SecondFactor::Refuse, SecondFactor::Claim)
}

/// Whether `code` has the form of a code: six ASCII digits.
fn is_code(code: &str) -> bool {
    code.len() == DIGITS && code.bytes().all(|byte| byte.is_ascii_digit())
}

/// The step `code` belongs to, if it is the secret's code for the step at `at` or one step
/// either side, and later than `last_used`. Each candidate step is compared in constant
/// time, and all of them are, so the time taken does not tell which one matched. Input that
/// is not six bytes is refused unread; six bytes that are not digits match no code.
pub(super) fn accepted_step(
    secret: &[u8],
    code: &str,
    at: DateTime<Utc>,
    last_used: Option<i64>,
) -> Option<i64> {
    let typed: &[u8; DIGITS] = code.as_bytes().try_into().ok()?;
    let now = step_at(at);
    let mut accepted = None;
    for step in now - DRIFT..=now + DRIFT {
        let Some(expected) = code_at(secret, step) else {
            continue;
        };
        let matched = bool::from(expected.ct_eq(typed));
        if matched && last_used.is_none_or(|last| step > last) {
            accepted = Some(step);
        }
    }
    accepted
}

/// The code for `step` as ASCII digits (RFC 4226 dynamic truncation), or `None` for a step
/// before the epoch.
fn code_at(secret: &[u8], step: i64) -> Option<[u8; DIGITS]> {
    let counter = u64::try_from(step).ok()?;
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("HMAC takes a key of any length");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[digest.len() - 1] & 0x0f);
    let word = u32::from_be_bytes([
        digest[offset],
        digest[offset + 1],
        digest[offset + 2],
        digest[offset + 3],
    ]);
    let mut value = (word & 0x7fff_ffff) % 1_000_000;
    let mut digits = [b'0'; DIGITS];
    for digit in digits.iter_mut().rev() {
        *digit = b'0' + u8::try_from(value % 10).expect("a decimal digit fits a byte");
        value /= 10;
    }
    Some(digits)
}

/// The `otpauth://` URI an authenticator app enrols from. It holds the secret: show it on
/// the operator's terminal once and never log it (LOG-007). Usernames are limited to
/// letters, digits, `.`, `_` and `-` (D-9), which need no percent-encoding.
pub(super) fn otpauth_uri(secret: &[u8], username: &str) -> Zeroizing<String> {
    let encoded = base32(secret);
    Zeroizing::new(format!(
        "otpauth://totp/{ISSUER}:{username}?secret={}&issuer={ISSUER}\
         &algorithm=SHA1&digits={DIGITS}&period={STEP_SECONDS}",
        encoded.as_str()
    ))
}

/// RFC 4648 base32 without padding, as authenticator apps read secrets.
fn base32(bytes: &[u8]) -> Zeroizing<String> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = Zeroizing::new(String::with_capacity(bytes.len().div_ceil(5) * 8));
    let mut buffer: u16 = 0;
    let mut bits = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u16::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(char::from(ALPHABET[usize::from((buffer >> bits) & 0x1f)]));
        }
    }
    if bits > 0 {
        out.push(char::from(
            ALPHABET[usize::from((buffer << (5 - bits)) & 0x1f)],
        ));
    }
    out
}
