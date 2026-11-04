//! Input types that never quote what they could not read (LOG-007, WP-06 H-4).
//!
//! async-graphql's derived parse errors quote the whole value they could not parse, and the
//! process log keeps the message of an error without a code. A check input can carry an HTTP
//! password, a bearer token or a header value, so every input that can hold one parses through
//! [`Opaque`], which replaces any parse error with a fixed `VALIDATION` error. A secret string
//! moves into a [`Secret`] at parse time ([`SecretText`]), without a copy.

use std::borrow::Cow;

use async_graphql::{
    InputType, InputValueError, InputValueResult, Value, registry::Registry, value,
};

use crate::credentials::Secret;

/// The field a parse error names: the input that could not be read is the whole argument.
const ARGUMENT: &str = "input";

/// An input of type `T` whose parse error holds none of the input.
pub(super) struct Opaque<T>(pub(super) T);

impl<T: InputType> InputType for Opaque<T> {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        T::type_name()
    }

    fn qualified_type_name() -> String {
        T::qualified_type_name()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        T::create_type_info(registry)
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        // The derived error is dropped unread: its message quotes the input.
        T::parse(value).map(Self).map_err(|_| unreadable())
    }

    /// An input that can hold a secret is never shown, not even as a default.
    fn to_value(&self) -> Value {
        Value::Null
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
}

/// A string that is a secret: held as a [`Secret`] from the moment it is parsed.
pub(super) struct SecretText(pub(super) Secret);

impl InputType for SecretText {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        <String as InputType>::type_name()
    }

    fn qualified_type_name() -> String {
        <String as InputType>::qualified_type_name()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        <String as InputType>::create_type_info(registry)
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        match value {
            Some(Value::String(text)) => Ok(Self(Secret::from(text))),
            _ => Err(unreadable()),
        }
    }

    fn to_value(&self) -> Value {
        Value::Null
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
}

/// A `VALIDATION` error that names the argument and quotes nothing of it.
fn unreadable<T: InputType>() -> InputValueError<T> {
    let message = "Check the shape of the check input against the API reference.";
    InputValueError::custom("the check input is not in the shape the API describes")
        .with_extension("code", "VALIDATION")
        .with_extension(
            "fields",
            Value::List(vec![value!({ "field": ARGUMENT, "message": message })]),
        )
}
