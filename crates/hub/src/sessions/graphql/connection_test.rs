//! `testConnection`'s input and result as K1 describes them (`ConnectionTestInput`,
//! `ConnectionTestResult`; DEV-013, LOG-007).
//!
//! async-graphql's derived parse error quotes the whole value it could not parse, credential
//! included (WP-06 H-4). So [`ConnectionTestInput`] parses through a derived shape and drops
//! that error unread, replacing it with a fixed one that quotes nothing.

use std::borrow::Cow;

use async_graphql::{
    ID, InputObject, InputType, InputValueError, InputValueResult, Object, Value,
    registry::Registry, value,
};

use crate::{
    devices::graphql::{SshCredential, SshSettingsInput},
    ids::DeviceId,
    sessions::{
        FieldError, HostKey,
        trust::{ConnectionTest, TestSubject, Unsaved},
    },
};

/// K1's default SSH port when the form sends no `ssh`.
const DEFAULT_PORT: u16 = 22;
/// Longest host accepted: a DNS name is at most 253 characters.
const MAX_HOST: usize = 253;

/// The schema shape of the argument. Only [`ConnectionTestInput`] parses into it.
#[derive(InputObject)]
#[graphql(name = "ConnectionTestInput")]
struct ConnectionTestShape {
    /// Test a saved device, or test unsaved values from the form (DEV-013).
    device_id: Option<ID>,
    host: Option<String>,
    ssh: Option<SshSettingsInput>,
    credential: Option<SshCredential>,
}

/// The parsed `input` argument of `testConnection`; the credential is already a secret.
pub(super) struct ConnectionTestInput(ConnectionTestShape);

impl InputType for ConnectionTestInput {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        ConnectionTestShape::type_name()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        ConnectionTestShape::create_type_info(registry)
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        // The derived error is dropped unread: its message quotes the input.
        ConnectionTestShape::parse(value)
            .map(Self)
            .map_err(|_| unreadable())
    }

    /// The input holds a secret, so it is never shown, not even in introspection.
    fn to_value(&self) -> Value {
        Value::Null
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
}

/// A `VALIDATION` error that names the argument and quotes nothing of it.
fn unreadable() -> InputValueError<ConnectionTestInput> {
    let message = "Send deviceId, or host with optional ssh settings, and optionally a \
                   password or a private key as the credential.";
    InputValueError::custom("the connection test input does not have K1's shape")
        .with_extension("code", "VALIDATION")
        .with_extension(
            "fields",
            Value::List(vec![value!({ "field": "input", "message": message })]),
        )
}

impl ConnectionTestInput {
    /// What to test, or every field to fix. Errors name fields, never values.
    pub(super) fn into_subject(self) -> Result<TestSubject, Vec<FieldError>> {
        let ConnectionTestShape {
            device_id,
            host,
            ssh,
            credential,
        } = self.0;
        let credential = credential
            .map(SshCredential::into_checked)
            .transpose()
            .map_err(|errors| {
                errors
                    .into_iter()
                    .map(|error| FieldError::new(error.field, error.message))
                    .collect::<Vec<_>>()
            })?;
        match (device_id, host, ssh) {
            (Some(id), None, None) => Ok(TestSubject::Device {
                id: device(&id)?,
                credential,
            }),
            (None, Some(host), ssh) => {
                let port = ssh
                    .as_ref()
                    .map_or(Some(i32::from(DEFAULT_PORT)), SshSettingsInput::port);
                match (checked_host(&host), checked_port(port)) {
                    (Ok(host), Ok(port)) => Ok(TestSubject::Unsaved(Unsaved { host, port })),
                    (host, port) => Err(host.err().into_iter().chain(port.err()).collect()),
                }
            }
            (None, None, _) => Err(vec![FieldError::new(
                "deviceId",
                "Send deviceId to test a saved device, or host to test the form's values.",
            )]),
            (Some(_), _, _) => Err(vec![FieldError::new(
                "deviceId",
                "Send deviceId alone, or host and ssh without deviceId.",
            )]),
        }
    }
}

fn device(id: &ID) -> Result<DeviceId, Vec<FieldError>> {
    id.parse()
        .map_err(|_| vec![FieldError::new("deviceId", "That device id is not valid.")])
}

/// A host name or IP literal: letters, digits, `.`, `-` and `:`, at most 253 characters. The
/// probe resolves it and checks every address (R-032).
fn checked_host(text: &str) -> Result<String, FieldError> {
    let host = text.trim();
    let shaped = !host.is_empty()
        && host.len() <= MAX_HOST
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-:".contains(&byte));
    if shaped {
        Ok(host.to_owned())
    } else {
        Err(FieldError::new(
            "host",
            "Enter a host name or IP address, without scheme, port or path.",
        ))
    }
}

fn checked_port(port: Option<i32>) -> Result<u16, FieldError> {
    port.and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port >= 1)
        .ok_or_else(|| FieldError::new("ssh.port", "The SSH port must be between 1 and 65535."))
}

#[Object(name = "ConnectionTestResult")]
impl ConnectionTest {
    async fn reachable(&self) -> bool {
        self.reachable
    }

    async fn authenticated(&self) -> bool {
        self.authenticated
    }

    /// Present when the host key is not yet accepted or has changed (SEC-006, SEC-007).
    async fn host_key(&self) -> Option<&HostKey> {
        self.host_key.as_ref()
    }

    async fn host_key_changed(&self) -> bool {
        self.host_key_changed
    }

    /// A fixed text for the operator; never the host's banner or error text.
    async fn message(&self) -> Option<&str> {
        Some(self.message)
    }
}
