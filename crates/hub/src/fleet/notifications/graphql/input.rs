//! The inputs of the channel mutations as K1 declares them (`NotificationChannelInput` and the
//! inputs below it). They only describe the shape; `form` turns them into a channel draft.
//!
//! Input types that never quote what they could not read (LOG-007): async-graphql's derived
//! parse errors quote the whole value they could not parse, and a channel input carries
//! webhook URLs, header values, ntfy topics and tokens, SMTP passwords and Apprise URLs and
//! keys. The argument parses through [`Opaque`], which replaces any parse error with a fixed
//! `VALIDATION` error, and every secret string moves into a [`Secret`] at parse time
//! ([`SecretText`]), without a copy. The checks inputs do the same.
//!
//! [`Opaque`] and [`SecretText`] are defence in depth. The `InternalErrors` extension already
//! replaces an argument error that echoes the argument with a fixed refusal before it leaves
//! the hub or reaches the log, so no schema shape or response tells these types from the
//! derived ones. They keep the secret out of the error value itself, so nothing that reads the
//! error before that boundary, or after a change to it, can see a secret.

use std::borrow::Cow;

use async_graphql::{
    InputObject, InputType, InputValueError, InputValueResult, OneofObject, Value,
    registry::Registry, value,
};

use super::enums::{AppriseMode, SmtpSecurity, WebhookMethod};
use crate::fleet::credentials::Secret;

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
    let message = "Check the shape of the channel input against the API reference.";
    InputValueError::custom("the channel input is not in the shape the API describes")
        .with_extension("code", "VALIDATION")
        .with_extension(
            "fields",
            Value::List(vec![value!({ "field": ARGUMENT, "message": message })]),
        )
}

/// K1's defaults, for an absent field and an explicit null alike (`form` reads them too).
pub(super) const DEFAULT_ON_BY_DEFAULT: bool = false;
pub(super) const DEFAULT_METHOD: WebhookMethod = WebhookMethod::Post;
/// The public ntfy server.
pub(super) const DEFAULT_NTFY: &str = "https://ntfy.sh";
pub(super) const DEFAULT_PRIORITY: i32 = 3;
pub(super) const DEFAULT_PORT: i32 = 587;
pub(super) const DEFAULT_SECURITY: SmtpSecurity = SmtpSecurity::StartTls;
pub(super) const DEFAULT_MODE: AppriseMode = AppriseMode::StatelessUrls;

#[derive(InputObject)]
#[graphql(name = "HttpHeaderInput")]
pub(super) struct HeaderShape {
    pub(super) name: String,
    pub(super) value: SecretText,
}

/// K1's one `HttpHeaderInput`, which the checks inputs declare too. Both describe the same
/// fields (the `k1` integration test compares them), so the schema keeps whichever registers
/// first and accepts the other under the same name instead of refusing to build.
pub(super) struct Header(pub(super) HeaderShape);

impl InputType for Header {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        HeaderShape::type_name()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        registry
            .ignore_name_conflicts
            .insert(HeaderShape::type_name().into_owned());
        HeaderShape::create_type_info(registry)
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        HeaderShape::parse(value)
            .map(Self)
            .map_err(|_| unreadable())
    }

    fn to_value(&self) -> Value {
        Value::Null
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
}

#[derive(InputObject)]
#[graphql(name = "WebhookSettingsInput")]
pub(super) struct WebhookShape {
    pub(super) url: Option<SecretText>,
    #[graphql(default_with = "Some(DEFAULT_METHOD)")]
    pub(super) method: Option<WebhookMethod>,
    pub(super) headers: Option<Vec<Header>>,
}

#[derive(InputObject)]
#[graphql(name = "NtfySettingsInput")]
pub(super) struct NtfyShape {
    #[graphql(default_with = "Some(DEFAULT_NTFY.to_owned())")]
    pub(super) server: Option<String>,
    pub(super) topic: Option<SecretText>,
    pub(super) token: Option<SecretText>,
    #[graphql(default_with = "Some(DEFAULT_PRIORITY)")]
    pub(super) priority: Option<i32>,
}

#[derive(InputObject)]
#[graphql(name = "EmailSettingsInput")]
pub(super) struct EmailShape {
    pub(super) host: String,
    #[graphql(default_with = "Some(DEFAULT_PORT)")]
    pub(super) port: Option<i32>,
    #[graphql(default_with = "Some(DEFAULT_SECURITY)")]
    pub(super) security: Option<SmtpSecurity>,
    pub(super) username: Option<String>,
    pub(super) password: Option<SecretText>,
    pub(super) from: String,
    pub(super) to: Vec<String>,
}

#[derive(InputObject)]
#[graphql(name = "AppriseSettingsInput")]
pub(super) struct AppriseShape {
    pub(super) server: String,
    #[graphql(default_with = "Some(DEFAULT_MODE)")]
    pub(super) mode: Option<AppriseMode>,
    pub(super) urls: Option<Vec<SecretText>>,
    pub(super) key: Option<SecretText>,
    #[graphql(default_with = "Some(Vec::new())")]
    pub(super) tags: Option<Vec<String>>,
}

#[derive(OneofObject)]
#[graphql(name = "ChannelSettingsInput")]
pub(super) enum SettingsShape {
    Webhook(WebhookShape),
    Ntfy(NtfyShape),
    Email(EmailShape),
    Apprise(AppriseShape),
}

#[derive(InputObject)]
#[graphql(name = "NotificationTemplateInput")]
pub(super) struct TemplateShape {
    pub(super) title: String,
    pub(super) body: String,
}

#[derive(InputObject)]
#[graphql(name = "NotificationChannelInput")]
pub(super) struct ChannelShape {
    pub(super) name: String,
    #[graphql(default_with = "Some(DEFAULT_ON_BY_DEFAULT)")]
    pub(super) on_by_default: Option<bool>,
    pub(super) settings: SettingsShape,
    pub(super) template: Option<TemplateShape>,
}
