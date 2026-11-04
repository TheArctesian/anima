//! The inputs of the check mutations as K1 declares them (`CreateCheckInput`,
//! `UpdateCheckInput` and the configuration inputs below them). They only describe the
//! shape; `form` turns them into domain values and checks the numbers.
//!
//! Input types that never quote what they could not read (LOG-007, WP-06 H-4).
//!
//! async-graphql's derived parse errors quote the whole value they could not parse, and the
//! process log keeps the message of an error without a code. A check input can carry an HTTP
//! password, a bearer token or a header value, so every input that can hold one parses through
//! [`Opaque`], which replaces any parse error with a fixed `VALIDATION` error. A secret string
//! moves into a [`Secret`] at parse time ([`SecretText`]), without a copy.

use std::borrow::Cow;

use async_graphql::{
    ID, InputObject, InputType, InputValueError, InputValueResult, MaybeUndefined, OneofObject,
    Value, registry::Registry, value,
};

use super::enums::{IpFamily, RecordType, RequestMethod};
use crate::{credentials::Secret, scalars::IpAddr};

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

#[derive(InputObject)]
#[graphql(name = "IcmpConfigInput")]
pub(super) struct IcmpShape {
    #[graphql(default_with = "Some(2000)")]
    pub(super) timeout_ms: Option<i32>,
    #[graphql(default_with = "Some(IpFamily::V4)")]
    pub(super) ip_version: Option<IpFamily>,
}

#[derive(InputObject)]
#[graphql(name = "TcpConfigInput")]
pub(super) struct TcpShape {
    pub(super) port: i32,
    #[graphql(default_with = "Some(5000)")]
    pub(super) timeout_ms: Option<i32>,
}

#[derive(InputObject)]
#[graphql(name = "TlsConfigInput")]
pub(super) struct TlsShape {
    #[graphql(default_with = "Some(443)")]
    pub(super) port: Option<i32>,
    pub(super) server_name: Option<String>,
    #[graphql(default_with = "Some(5000)")]
    pub(super) timeout_ms: Option<i32>,
}

#[derive(InputObject)]
#[graphql(name = "HttpHeaderInput")]
pub(super) struct HeaderShape {
    pub(super) name: String,
    pub(super) value: SecretText,
}

#[derive(InputObject)]
#[graphql(name = "StatusRangeInput")]
pub(super) struct StatusRangeShape {
    pub(super) from: i32,
    pub(super) to: i32,
}

#[derive(InputObject)]
#[graphql(name = "BasicAuthInput")]
pub(super) struct BasicShape {
    pub(super) username: String,
    pub(super) password: SecretText,
}

#[derive(OneofObject)]
#[graphql(name = "HttpAuthInput")]
pub(super) enum AuthShape {
    Basic(Opaque<BasicShape>),
    BearerToken(SecretText),
}

#[derive(InputObject)]
#[graphql(name = "HttpConfigInput")]
pub(super) struct HttpShape {
    pub(super) url: String,
    #[graphql(default_with = "Some(RequestMethod::Get)")]
    pub(super) method: Option<RequestMethod>,
    #[graphql(default_with = "Some(Vec::new())")]
    pub(super) headers: Option<Vec<Opaque<HeaderShape>>>,
    pub(super) body: Option<String>,
    #[graphql(default_with = "Some(10)")]
    pub(super) max_redirects: Option<i32>,
    #[graphql(default_with = "Some(vec![StatusRangeShape { from: 200, to: 299 }])")]
    pub(super) accepted_status: Option<Vec<StatusRangeShape>>,
    pub(super) keyword: Option<String>,
    pub(super) json_query: Option<String>,
    pub(super) json_expected: Option<String>,
    pub(super) auth: Option<Opaque<AuthShape>>,
    #[graphql(default_with = "Some(true)")]
    pub(super) verify_tls: Option<bool>,
    #[graphql(default_with = "Some(10000)")]
    pub(super) timeout_ms: Option<i32>,
}

#[derive(InputObject)]
#[graphql(name = "DnsConfigInput")]
pub(super) struct DnsShape {
    pub(super) name: String,
    #[graphql(default_with = "Some(RecordType::A)")]
    pub(super) record_type: Option<RecordType>,
    pub(super) resolver: Option<IpAddr>,
    pub(super) expected: Option<String>,
}

#[derive(InputObject)]
#[graphql(name = "GroupConfigInput")]
pub(super) struct GroupShape {
    pub(super) child_ids: Vec<ID>,
}

#[derive(OneofObject)]
#[graphql(name = "CheckConfigInput")]
pub(super) enum ConfigShape {
    Icmp(IcmpShape),
    Tcp(TcpShape),
    Tls(TlsShape),
    Http(Opaque<HttpShape>),
    Dns(DnsShape),
    Push(bool),
    Group(GroupShape),
}

#[derive(InputObject)]
#[graphql(name = "CreateCheckInput")]
pub(super) struct CreateShape {
    pub(super) device_id: ID,
    pub(super) parent_id: Option<ID>,
    pub(super) name: String,
    pub(super) config: Opaque<ConfigShape>,
    #[graphql(default_with = "Some(60)")]
    pub(super) interval_seconds: Option<i32>,
    #[graphql(default_with = "Some(3)")]
    pub(super) retries: Option<i32>,
    #[graphql(default_with = "Some(20)")]
    pub(super) retry_interval_seconds: Option<i32>,
    #[graphql(default_with = "Some(1000)")]
    pub(super) degraded_threshold_ms: Option<i32>,
    #[graphql(default_with = "Some(false)")]
    pub(super) invert: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "UpdateCheckInput")]
pub(super) struct UpdateShape {
    /// Absent keeps the group; null removes the check from its group.
    pub(super) parent_id: MaybeUndefined<ID>,
    pub(super) name: Option<String>,
    pub(super) config: Option<Opaque<ConfigShape>>,
    pub(super) interval_seconds: Option<i32>,
    pub(super) retries: Option<i32>,
    pub(super) retry_interval_seconds: Option<i32>,
    pub(super) degraded_threshold_ms: Option<i32>,
    pub(super) invert: Option<bool>,
}
