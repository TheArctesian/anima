//! Validation of operator input for sites and devices (DEV-010, VPN-001).
//!
//! Every function is pure. Validators report all field errors at once so the form can show them together.

use std::net::IpAddr;

use super::model::{
    DeviceChange, DeviceChangeInput, NewDevice, NewDeviceInput, NewSite, NewSiteInput, SshInput,
    SshSettings,
};

const NAME_MAX: usize = 64;
const TAG_MAX: usize = 32;
const HOST_MAX: usize = 253;
const LABEL_MAX: usize = 63;
const DEFAULT_KIND: &str = "server";
const DEFAULT_SSH_PORT: i32 = 22;

/// One input field the operator must fix. `field` is the GraphQL input field name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    /// The GraphQL input field in camelCase, as the client form names it; nested SSH
    /// fields are written `ssh.port` and `ssh.username`.
    pub field: &'static str,
    pub message: String,
}

impl FieldError {
    pub(super) fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Collects field errors while the remaining fields are still checked.
#[derive(Default)]
struct Problems(Vec<FieldError>);

impl Problems {
    fn field<T>(&mut self, checked: Result<T, FieldError>) -> Option<T> {
        checked.map_err(|error| self.0.push(error)).ok()
    }

    fn fields<T>(&mut self, checked: Result<T, Vec<FieldError>>) -> Option<T> {
        checked.map_err(|errors| self.0.extend(errors)).ok()
    }

    fn finish<T>(self, value: Option<T>) -> Result<T, Vec<FieldError>> {
        match value {
            Some(value) if self.0.is_empty() => Ok(value),
            _ => Err(self.0),
        }
    }
}

/// Checks a new device (DEV-010).
///
/// # Errors
/// Every invalid field, all at once.
#[cfg_attr(not(test), expect(dead_code, reason = "used by the devices GraphQL"))]
pub(super) fn validate_new_device(input: &NewDeviceInput) -> Result<NewDevice, Vec<FieldError>> {
    let mut problems = Problems::default();
    let name = problems.field(device_name(&input.name));
    let host = problems.field(host(&input.host));
    let kind = problems.field(kind(&input.kind));
    let tags = problems.field(tags(&input.tags));
    let ssh = problems.fields(
        input
            .ssh
            .as_ref()
            .map_or_else(|| Ok(SshSettings::default()), ssh_settings),
    );
    let checked = match (name, host, kind, tags, ssh) {
        (Some(name), Some(host), Some(kind), Some(tags), Some(ssh)) => Some(NewDevice {
            site: input.site,
            name,
            host,
            kind,
            os: input.os,
            tags,
            ssh,
        }),
        _ => None,
    };
    problems.finish(checked)
}

/// Checks a device update; only the fields that are present are checked (DEV-010).
///
/// # Errors
/// Every invalid field, all at once.
#[cfg_attr(not(test), expect(dead_code, reason = "used by the devices GraphQL"))]
pub(super) fn validate_change(input: &DeviceChangeInput) -> Result<DeviceChange, Vec<FieldError>> {
    let mut problems = Problems::default();
    let name = input
        .name
        .as_deref()
        .map(device_name)
        .and_then(|c| problems.field(c));
    let host = input
        .host
        .as_deref()
        .map(host)
        .and_then(|c| problems.field(c));
    let kind = input
        .kind
        .as_deref()
        .map(kind)
        .and_then(|c| problems.field(c));
    let tags = input
        .tags
        .as_deref()
        .map(tags)
        .and_then(|c| problems.field(c));
    let ssh = input
        .ssh
        .as_ref()
        .map(ssh_settings)
        .and_then(|c| problems.fields(c));
    let checked = DeviceChange {
        site: input.site,
        name,
        host,
        kind,
        os: input.os,
        tags,
        maintenance: input.maintenance,
        ssh,
    };
    problems.finish(Some(checked))
}

/// Checks a new site (VPN-001). The subnet arrives parsed as a `Cidr`, which rejects host bits;
/// overlap with other sites is the database's job.
///
/// # Errors
/// Every invalid field, all at once.
#[cfg_attr(not(test), expect(dead_code, reason = "used by the devices GraphQL"))]
pub(super) fn validate_new_site(input: &NewSiteInput) -> Result<NewSite, Vec<FieldError>> {
    let mut problems = Problems::default();
    let name = problems.field(site_name(&input.name));
    let checked = name.map(|name| NewSite {
        name,
        subnet: input.subnet,
    });
    problems.finish(checked)
}

fn length_between(text: &str, max: usize) -> bool {
    (1..=max).contains(&text.chars().count())
}

fn device_name(text: &str) -> Result<String, FieldError> {
    name(text, "name")
}

fn site_name(text: &str) -> Result<String, FieldError> {
    name(text, "site name")
}

fn name(text: &str, noun: &str) -> Result<String, FieldError> {
    let name = text.trim();
    if length_between(name, NAME_MAX) {
        Ok(name.to_owned())
    } else {
        Err(FieldError::new(
            "name",
            format!("The {noun} must be 1 to {NAME_MAX} characters."),
        ))
    }
}

fn kind(text: &str) -> Result<String, FieldError> {
    let kind = text.trim();
    if kind.is_empty() {
        Ok(DEFAULT_KIND.to_owned())
    } else if length_between(kind, NAME_MAX) {
        Ok(kind.to_owned())
    } else {
        Err(FieldError::new(
            "kind",
            format!("The kind must be at most {NAME_MAX} characters."),
        ))
    }
}

/// An IP address or an RFC 1123 host name, without scheme, port or path (DEV-010).
fn host(text: &str) -> Result<String, FieldError> {
    let host = text.trim();
    let reject = |message: &str| Err(FieldError::new("host", message));
    if host.is_empty() {
        return reject("Enter a host name or IP address.");
    }
    if host.parse::<IpAddr>().is_ok() {
        return Ok(host.to_owned());
    }
    if host.contains("://") {
        return reject(
            "Enter only the host name or IP address, without http:// or another scheme.",
        );
    }
    if host.contains(':') {
        return reject(
            "Enter only the host name or IP address, without a port. Set the SSH port under SSH.",
        );
    }
    if host.contains('/') {
        return reject("Enter only the host name or IP address, without a path.");
    }
    if host.chars().any(char::is_whitespace) {
        return reject("The host cannot contain spaces.");
    }
    let name = host.strip_suffix('.').unwrap_or(host);
    if name.len() > HOST_MAX {
        return reject("The host name can be at most 253 characters.");
    }
    if !name.split('.').all(is_host_label) {
        return reject(
            "Use letters, digits and hyphens in labels of 1 to 63 characters separated by dots; a label cannot start or end with a hyphen.",
        );
    }
    let last = name.rsplit('.').next().unwrap_or(name);
    if last.bytes().all(|byte| byte.is_ascii_digit()) {
        return reject(
            "That is not a valid IP address, and a host name cannot end in digits only.",
        );
    }
    Ok(host.to_owned())
}

fn is_host_label(label: &str) -> bool {
    (1..=LABEL_MAX).contains(&label.len())
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// Trimmed tags without repeats, in first-seen order.
fn tags(raw: &[String]) -> Result<Vec<String>, FieldError> {
    let mut kept: Vec<String> = Vec::with_capacity(raw.len());
    for (index, tag) in raw.iter().map(|tag| tag.trim()).enumerate() {
        let position = index + 1;
        if tag.is_empty() {
            return Err(FieldError::new("tags", format!("Tag {position} is empty.")));
        }
        if !length_between(tag, TAG_MAX) {
            return Err(FieldError::new(
                "tags",
                format!("Tag {position} is longer than {TAG_MAX} characters."),
            ));
        }
        if !kept.iter().any(|seen| seen == tag) {
            kept.push(tag.to_owned());
        }
    }
    Ok(kept)
}

fn ssh_settings(input: &SshInput) -> Result<SshSettings, Vec<FieldError>> {
    let mut problems = Problems::default();
    let port = problems.field(ssh_port(input.port.unwrap_or(DEFAULT_SSH_PORT)));
    let username = problems.field(ssh_username(&input.username, input.enabled));
    let checked = port.zip(username).map(|(port, username)| SshSettings {
        enabled: input.enabled,
        port,
        username,
        auth: input.auth,
    });
    problems.finish(checked)
}

fn ssh_port(port: i32) -> Result<u16, FieldError> {
    u16::try_from(port)
        .ok()
        .filter(|port| *port >= 1)
        .ok_or_else(|| FieldError::new("ssh.port", "The SSH port must be between 1 and 65535."))
}

fn ssh_username(text: &str, enabled: bool) -> Result<String, FieldError> {
    let username = text.trim();
    if enabled && username.is_empty() {
        return Err(FieldError::new("ssh.username", "Enter the SSH user name."));
    }
    if username.chars().any(char::is_whitespace) {
        return Err(FieldError::new(
            "ssh.username",
            "The SSH user name cannot contain spaces.",
        ));
    }
    Ok(username.to_owned())
}
