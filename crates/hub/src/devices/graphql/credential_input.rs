//! The credential argument of `setSshCredential` (K1 `SshCredentialInput`) and the checks on
//! it (DEV-012, SEC-005, LOG-007).
//!
//! async-graphql's derived parse errors quote the whole value they could not parse, and the
//! process log keeps the message of an error without a code. A bare string sent as the
//! credential would reach the log. So [`SshCredential`] parses the argument and replaces any
//! parse error with a fixed one that holds no input; each string moves into a [`Secret`]
//! without a copy.

use std::borrow::Cow;

use async_graphql::{
    InputObject, InputType, InputValueError, InputValueResult, OneofObject, Value,
    registry::Registry, value,
};

use crate::{
    credentials::{Secret, SshSecret},
    devices::FieldError,
};

/// The schema shape of the argument. Only [`SshCredential`] parses into it.
#[derive(OneofObject)]
#[graphql(name = "SshCredentialInput")]
enum SshCredentialInput {
    Password(String),
    PrivateKey(PrivateKeyInput),
}

#[derive(InputObject)]
#[graphql(name = "PrivateKeyInput")]
struct PrivateKeyInput {
    key: String,
    passphrase: Option<String>,
}

/// The parsed `credential` argument, already held as secrets.
pub struct SshCredential(pub(super) SshSecret);

impl SshCredential {
    /// The secret, once it passes the checks `setSshCredential` applies. For
    /// `ConnectionTestInput.credential` (WP-10, WP-06 H-4).
    ///
    /// # Errors
    /// Every credential field to fix; no error holds the value or its length.
    pub fn into_checked(self) -> Result<SshSecret, Vec<FieldError>> {
        checked_secret(&self.0)?;
        Ok(self.0)
    }
}

/// The field a parse error names: the input that could not be read is the whole argument.
const ARGUMENT: &str = "credential";

impl InputType for SshCredential {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        SshCredentialInput::type_name()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        SshCredentialInput::create_type_info(registry)
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        // The derived error is dropped unread: its message quotes the input.
        let input = SshCredentialInput::parse(value).map_err(|_| unreadable())?;
        Ok(Self(match input {
            SshCredentialInput::Password(password) => SshSecret::Password(Secret::from(password)),
            SshCredentialInput::PrivateKey(PrivateKeyInput { key, passphrase }) => {
                SshSecret::PrivateKey {
                    key: Secret::from(key),
                    passphrase: passphrase.map(Secret::from),
                }
            }
        }))
    }

    /// A secret is never shown, not even in introspection.
    fn to_value(&self) -> Value {
        Value::Null
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
}

/// A `VALIDATION` error that names the argument and quotes nothing of it.
fn unreadable() -> InputValueError<SshCredential> {
    let message = "Send a password, or a private key with an optional passphrase.";
    InputValueError::custom("the credential is not a password or a private key")
        .with_extension("code", "VALIDATION")
        .with_extension(
            "fields",
            Value::List(vec![value!({ "field": ARGUMENT, "message": message })]),
        )
}

/// Which kind of secret was stored; what the audit entry may say about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CredentialKind {
    Password,
    PrivateKey { passphrase: bool },
}

/// Largest private key accepted: an RSA 16384-bit key in PEM is under 13 KiB.
const MAX_KEY_BYTES: usize = 16 * 1024;
/// Largest password or passphrase accepted.
const MAX_TEXT_BYTES: usize = 1024;

const PASSWORD: &str = "credential.password";
const KEY: &str = "credential.privateKey.key";
const PASSPHRASE: &str = "credential.privateKey.passphrase";

/// The kind of the credential, or the fields to fix. Errors name the field, never the value
/// or its length.
pub(super) fn checked_secret(secret: &SshSecret) -> Result<CredentialKind, Vec<FieldError>> {
    let problems: Vec<_> = match secret {
        SshSecret::Password(password) => text_problem(PASSWORD, password).into_iter().collect(),
        SshSecret::PrivateKey { key, passphrase } => key_problem(key)
            .into_iter()
            .chain(
                passphrase
                    .as_ref()
                    .and_then(|p| text_problem(PASSPHRASE, p)),
            )
            .collect(),
    };
    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(match secret {
        SshSecret::Password(_) => CredentialKind::Password,
        SshSecret::PrivateKey { passphrase, .. } => CredentialKind::PrivateKey {
            passphrase: passphrase.is_some(),
        },
    })
}

fn text_problem(field: &'static str, secret: &Secret) -> Option<FieldError> {
    let bytes = secret.expose();
    if bytes.len() > MAX_TEXT_BYTES {
        Some(FieldError::new(field, "Use at most 1024 bytes."))
    } else if bytes.trim_ascii().is_empty() {
        Some(FieldError::new(field, "Enter a value."))
    } else {
        None
    }
}

const BEGIN: &[u8] = b"-----BEGIN ";
const PRIVATE_END: &[u8] = b"PRIVATE KEY-----";

fn key_problem(key: &Secret) -> Option<FieldError> {
    let bytes = key.expose();
    // Size first, so an oversized value is never searched.
    if bytes.len() > MAX_KEY_BYTES {
        return Some(FieldError::new(KEY, "Use a private key of at most 16 KiB."));
    }
    let text = bytes.trim_ascii();
    if text.is_empty() {
        return Some(FieldError::new(KEY, "Enter a private key."));
    }
    // The first line must be the key's own header, `-----BEGIN <kind> PRIVATE KEY-----`, so a
    // public key with a private key marker further down is refused.
    let header = text
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default()
        .trim_ascii_end();
    // Between the two, only the key format: `OPENSSH `, `RSA `, `ENCRYPTED `, or nothing.
    let marked = header.len() >= BEGIN.len() + PRIVATE_END.len()
        && header.starts_with(BEGIN)
        && header.ends_with(PRIVATE_END)
        && header[BEGIN.len()..header.len() - PRIVATE_END.len()]
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b' ');
    (!marked).then(|| {
        FieldError::new(
            KEY,
            "Paste a private key whose first line is -----BEGIN … PRIVATE KEY-----.",
        )
    })
}
