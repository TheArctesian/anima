//! The credential argument of `setRdpCredential` and `setVncCredential` (K1
//! `PasswordCredentialInput`) and the check on it (DEV-011, SEC-005, LOG-007).
//!
//! Parsed as [`super::SshCredential`] is: a derived parse error quotes the value, so it is
//! dropped and replaced by a fixed one, and the string moves into a [`Secret`] without a copy.

use std::borrow::Cow;

use async_graphql::{
    InputObject, InputType, InputValueError, InputValueResult, Value, registry::Registry, value,
};

use super::{ARGUMENT, PASSWORD, text_problem};
use crate::fleet::{credentials::Secret, devices::FieldError};

/// The schema shape of the argument. Only [`PasswordCredential`] parses into it.
#[derive(InputObject)]
#[graphql(name = "PasswordCredentialInput")]
struct PasswordCredentialInput {
    password: String,
}

/// The parsed `credential` argument, already held as a secret.
pub struct PasswordCredential(Secret);

impl PasswordCredential {
    /// The password, once it is not blank and at most 1024 bytes.
    ///
    /// # Errors
    /// The `credential.password` field to fix; no error holds the value or its length.
    pub fn into_checked(self) -> Result<Secret, Vec<FieldError>> {
        match text_problem(PASSWORD, &self.0) {
            Some(problem) => Err(vec![problem]),
            None => Ok(self.0),
        }
    }
}

impl InputType for PasswordCredential {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        PasswordCredentialInput::type_name()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        PasswordCredentialInput::create_type_info(registry)
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        // The derived error is dropped unread: its message quotes the input.
        let PasswordCredentialInput { password } =
            PasswordCredentialInput::parse(value).map_err(|_| unreadable())?;
        Ok(Self(Secret::from(password)))
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
fn unreadable() -> InputValueError<PasswordCredential> {
    InputValueError::custom("the credential is not a password")
        .with_extension("code", "VALIDATION")
        .with_extension(
            "fields",
            Value::List(vec![
                value!({ "field": ARGUMENT, "message": "Send a password." }),
            ]),
        )
}
