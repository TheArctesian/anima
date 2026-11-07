//! `testConnection`'s input and result as K1 describes them (`ConnectionTestInput`,
//! `ConnectionTestResult`; DEV-013, SEC-007, LOG-007).
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
    devices::{
        self,
        graphql::{
            PasswordCredential, RdpSettingsInput, SshCredential, SshSettingsInput, VncSettingsInput,
        },
    },
    ids::DeviceId,
    sessions::{
        FieldError, HostKey, Protocol, ServerCertificate,
        trust::{ConnectionTest, Desktop, DesktopSettings, DesktopSubject, TestSubject, Unsaved},
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
    #[graphql(default_with = "Some(Protocol::Ssh)")]
    protocol: Option<Protocol>,
    rdp: Option<RdpSettingsInput>,
    vnc: Option<VncSettingsInput>,
    /// Accepted for the form's sake and dropped unread: the hub does not sign in during an
    /// RDP or VNC test.
    password: Option<PasswordCredential>,
}

/// The parsed `input` argument of `testConnection`; the credential is already a secret.
pub(in crate::sessions) struct ConnectionTestInput(ConnectionTestShape);

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
    let message = "Send deviceId, or host with optional settings for the protocol, and for SSH \
                   optionally a password or a private key as the credential.";
    InputValueError::custom("the connection test input does not have K1's shape")
        .with_extension("code", "VALIDATION")
        .with_extension(
            "fields",
            Value::List(vec![value!({ "field": "input", "message": message })]),
        )
}

impl ConnectionTestInput {
    /// What to test, or every field to fix. Errors name fields, never values.
    pub(in crate::sessions) fn into_subject(self) -> Result<TestSubject, Vec<FieldError>> {
        let ConnectionTestShape {
            device_id,
            host,
            ssh,
            credential,
            protocol,
            rdp,
            vnc,
            password,
        } = self.0;
        let protocol = protocol.unwrap_or(Protocol::Ssh);
        // `password` is never read and drops here, zeroed: no credential leaves the hub in an
        // RDP or VNC test.
        let stray = [
            ("ssh", ssh.is_some(), protocol == Protocol::Ssh),
            (
                "credential",
                credential.is_some(),
                protocol == Protocol::Ssh,
            ),
            ("rdp", rdp.is_some(), protocol == Protocol::Rdp),
            ("vnc", vnc.is_some(), protocol == Protocol::Vnc),
            ("password", password.is_some(), protocol != Protocol::Ssh),
        ];
        drop(password);
        let stray: Vec<FieldError> = stray
            .into_iter()
            .filter(|(_, sent, allowed)| *sent && !allowed)
            .map(|(field, _, _)| FieldError::new(field, STRAY))
            .collect();
        if !stray.is_empty() {
            return Err(stray);
        }
        let settings = ssh.is_some() || rdp.is_some() || vnc.is_some();
        if device_id.is_some() && (host.is_some() || settings) {
            return Err(vec![subject_error(true)]);
        }
        match protocol {
            Protocol::Ssh => ssh_subject(device_id, host, ssh.as_ref(), credential),
            Protocol::Rdp => desktop_subject(device_id, host, Desktop::Rdp, || rdp_settings(rdp)),
            Protocol::Vnc => desktop_subject(device_id, host, Desktop::Vnc, || vnc_settings(vnc)),
        }
    }
}

const STRAY: &str = "This field belongs to another protocol. Send ssh and credential with SSH, \
                     rdp and password with RDP, vnc and password with VNC.";

fn from_devices(errors: Vec<devices::FieldError>) -> Vec<FieldError> {
    errors
        .into_iter()
        .map(|error| FieldError::new(error.field, error.message))
        .collect()
}

fn ssh_subject(
    device_id: Option<ID>,
    host: Option<String>,
    ssh: Option<&SshSettingsInput>,
    credential: Option<SshCredential>,
) -> Result<TestSubject, Vec<FieldError>> {
    let credential = credential
        .map(SshCredential::into_checked)
        .transpose()
        .map_err(from_devices)?;
    match (device_id, host) {
        (Some(id), None) => Ok(TestSubject::Device {
            id: device(&id)?,
            credential,
        }),
        (None, Some(host)) => {
            let port = ssh.map_or(Some(i32::from(DEFAULT_PORT)), SshSettingsInput::port);
            match (checked_host(&host), checked_port(port)) {
                (Ok(host), Ok(port)) => Ok(TestSubject::Unsaved(Unsaved { host, port })),
                (host, port) => Err(host.err().into_iter().chain(port.err()).collect()),
            }
        }
        (device_id, host) => Err(vec![subject_error(device_id.is_some() && host.is_some())]),
    }
}

/// An RDP or VNC subject: a saved device alone, or a host with the form's settings, which
/// `settings` checks only for a host.
fn desktop_subject(
    device_id: Option<ID>,
    host: Option<String>,
    protocol: Desktop,
    settings: impl FnOnce() -> Result<DesktopSettings, Vec<FieldError>>,
) -> Result<TestSubject, Vec<FieldError>> {
    let subject = match (device_id, host) {
        (Some(id), None) => DesktopSubject::Device {
            id: device(&id)?,
            protocol,
        },
        (None, Some(host)) => match (checked_host(&host), settings()) {
            (Ok(host), Ok(settings)) => DesktopSubject::Unsaved { host, settings },
            (host, settings) => {
                let host = host.err().into_iter();
                return Err(host.chain(settings.err().into_iter().flatten()).collect());
            }
        },
        (device_id, host) => {
            return Err(vec![subject_error(device_id.is_some() && host.is_some())]);
        }
    };
    Ok(TestSubject::Desktop(subject))
}

/// The form's RDP settings, K1's defaults when none are sent.
fn rdp_settings(rdp: Option<RdpSettingsInput>) -> Result<DesktopSettings, Vec<FieldError>> {
    let settings = rdp
        .map(RdpSettingsInput::validated)
        .transpose()
        .map_err(from_devices)?
        .unwrap_or_default();
    Ok(DesktopSettings::Rdp {
        port: settings.port,
        security: settings.security,
    })
}

/// The form's VNC port, K1's default when no settings are sent.
fn vnc_settings(vnc: Option<VncSettingsInput>) -> Result<DesktopSettings, Vec<FieldError>> {
    let settings = vnc
        .map(VncSettingsInput::validated)
        .transpose()
        .map_err(from_devices)?
        .unwrap_or_default();
    Ok(DesktopSettings::Vnc {
        port: settings.port,
    })
}

fn subject_error(both: bool) -> FieldError {
    let message = if both {
        "Send deviceId alone, or host and its settings without deviceId."
    } else {
        "Send deviceId to test a saved device, or host to test the form's values."
    };
    FieldError::new("deviceId", message)
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

    /// Present when no server certificate is accepted (`certificateChanged` false) or the RDP
    /// server presented another (`certificateChanged` true); null when it is the accepted one
    /// (SEC-007).
    async fn server_certificate(&self) -> Option<&ServerCertificate> {
        self.server_certificate.as_ref()
    }

    async fn certificate_changed(&self) -> bool {
        self.certificate_changed
    }

    /// A fixed text for the operator; never the host's banner or error text.
    async fn message(&self) -> Option<&str> {
        Some(self.message)
    }
}
