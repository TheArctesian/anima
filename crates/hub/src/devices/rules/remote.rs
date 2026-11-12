//! Validation of the remote access settings of a device (RA-030..033, D-2, D-6).
//!
//! Field paths are `rdp.port`, `rdp.username`, `rdp.domain`, `vnc.port`, `vnc.username`,
//! `macAddress`, `sessionLimit` and `metricsSource`; the wake relay check names `deviceId`.

use std::num::NonZeroU8;

use super::{FieldError, Problems};
use crate::text::is_plain;
use crate::{
    devices::{
        Device,
        mac::{MacAddress, MacAddressError},
        remote::{
            MetricsSource, RdpInput, RdpSettings, RemoteAccess, RemoteChange, RemoteForm, VncInput,
            VncSettings,
        },
        text::user_name,
    },
    ids::SiteId,
};

const DOMAIN_MAX: usize = 255;
const SESSION_LIMIT_MAX: u8 = 100;

/// Checks the remote access part of a new device; settings left out take the K1 defaults.
///
/// # Errors
/// Every invalid field, all at once.
pub(in crate::devices) fn new_remote(form: &RemoteForm) -> Result<RemoteAccess, Vec<FieldError>> {
    let mut problems = Problems::default();
    let rdp = form
        .rdp
        .as_ref()
        .map_or_else(|| Ok(RdpSettings::default()), rdp_settings);
    let vnc = form
        .vnc
        .as_ref()
        .map_or_else(|| Ok(VncSettings::default()), vnc_settings);
    let rdp = problems.fields(rdp);
    let vnc = problems.fields(vnc);
    let mac_address = problems.field(form.mac_address.as_deref().map_or(Ok(None), mac_address));
    let session_limit = problems.field(form.session_limit.map_or(Ok(None), new_session_limit));
    let metrics_source = problems.field(metrics_source(form.metrics_source.unwrap_or_default()));
    let checked = rdp
        .zip(vnc)
        .zip(mac_address)
        .zip(session_limit)
        .zip(metrics_source)
        .map(
            |((((rdp, vnc), mac_address), session_limit), metrics_source)| RemoteAccess {
                rdp,
                vnc,
                mac_address,
                metrics_source,
                transfer: form.transfer.unwrap_or_default(),
                session_limit,
            },
        );
    problems.finish(checked)
}

/// Checks the remote access part of a device update; absent parts stay as they are (D-2).
///
/// # Errors
/// Every invalid field, all at once.
pub(in crate::devices) fn remote_change(
    form: &RemoteForm,
) -> Result<RemoteChange, Vec<FieldError>> {
    let mut problems = Problems::default();
    let rdp = form
        .rdp
        .as_ref()
        .map(rdp_settings)
        .and_then(|c| problems.fields(c));
    let vnc = form
        .vnc
        .as_ref()
        .map(vnc_settings)
        .and_then(|c| problems.fields(c));
    let mac_address = form
        .mac_address
        .as_deref()
        .map(mac_address)
        .and_then(|c| problems.field(c));
    let session_limit = form
        .session_limit
        .map(changed_session_limit)
        .and_then(|c| problems.field(c));
    let metrics_source = form
        .metrics_source
        .map(metrics_source)
        .and_then(|c| problems.field(c));
    problems.finish(Some(RemoteChange {
        rdp,
        vnc,
        mac_address,
        metrics_source,
        transfer: form.transfer,
        session_limit,
    }))
}

pub(in crate::devices) fn rdp_settings(input: &RdpInput) -> Result<RdpSettings, Vec<FieldError>> {
    let mut problems = Problems::default();
    let port = problems.field(port(
        input.port,
        RdpSettings::default().port,
        "rdp.port",
        "RDP",
    ));
    let username = problems.field(user_name(
        &input.username,
        input.enabled,
        "rdp.username",
        "RDP user name",
    ));
    let domain = problems.field(domain(input.domain.as_deref()));
    let checked = port
        .zip(username)
        .zip(domain)
        .map(|((port, username), domain)| RdpSettings {
            enabled: input.enabled,
            port,
            username,
            domain,
            security: input.security,
            ignore_certificate: input.ignore_certificate,
            resize: input.resize,
            audio_out: input.audio_out,
            microphone: input.microphone,
            shared_drive: input.shared_drive,
        });
    problems.finish(checked)
}

pub(in crate::devices) fn vnc_settings(input: &VncInput) -> Result<VncSettings, Vec<FieldError>> {
    let mut problems = Problems::default();
    let port = problems.field(port(
        input.port,
        VncSettings::default().port,
        "vnc.port",
        "VNC",
    ));
    let username = problems.field(
        input
            .username
            .as_deref()
            .map(|text| user_name(text, false, "vnc.username", "VNC user name"))
            .transpose()
            .map(|name| name.filter(|name| !name.is_empty())),
    );
    let checked = port.zip(username).map(|(port, username)| VncSettings {
        enabled: input.enabled,
        port,
        username,
    });
    problems.finish(checked)
}

fn port(
    raw: Option<i32>,
    default: u16,
    field: &'static str,
    protocol: &str,
) -> Result<u16, FieldError> {
    raw.map_or(Ok(default), |port| {
        u16::try_from(port)
            .ok()
            .filter(|port| *port >= 1)
            .ok_or_else(|| {
                FieldError::new(
                    field,
                    format!("The {protocol} port must be between 1 and 65535."),
                )
            })
    })
}

/// The Windows domain: trimmed, `None` when empty, at most 255 characters, plain, no spaces.
fn domain(text: Option<&str>) -> Result<Option<String>, FieldError> {
    let domain = text.map_or("", str::trim);
    let reject = |message: &str| Err(FieldError::new("rdp.domain", message));
    if domain.chars().count() > DOMAIN_MAX {
        return reject("The domain can be at most 255 characters.");
    }
    if !is_plain(domain) {
        return reject("The domain cannot contain control characters or text-direction marks.");
    }
    if domain.chars().any(char::is_whitespace) {
        return reject("The domain cannot contain spaces.");
    }
    Ok((!domain.is_empty()).then(|| domain.to_owned()))
}

/// The source the operator chose. `Agent` is refused: only enrolling an agent sets it, so a
/// device never waits for an agent that has no token (MET-011, R-092 D-7).
fn metrics_source(source: MetricsSource) -> Result<MetricsSource, FieldError> {
    if source == MetricsSource::Agent {
        return Err(FieldError::new(
            "metricsSource",
            "Install the agent to use it: enrolling it sets this source.",
        ));
    }
    Ok(source)
}

const SESSION_LIMIT_MESSAGE: &str = "Enter a number from 1 to 100, or leave it empty for no limit.";

/// A new device's limit: 1 to 100 (RA-036).
fn new_session_limit(limit: i32) -> Result<Option<NonZeroU8>, FieldError> {
    u8::try_from(limit)
        .ok()
        .filter(|limit| (1..=SESSION_LIMIT_MAX).contains(limit))
        .and_then(NonZeroU8::new)
        .map(Some)
        .ok_or_else(|| FieldError::new("sessionLimit", SESSION_LIMIT_MESSAGE))
}

/// An update's limit: 0 clears it, 1 to 100 sets it (RA-036).
fn changed_session_limit(limit: i32) -> Result<Option<NonZeroU8>, FieldError> {
    if limit == 0 {
        Ok(None)
    } else {
        new_session_limit(limit)
    }
}

/// `None` for empty text (no address); otherwise the parsed unicast address (D-6).
fn mac_address(text: &str) -> Result<Option<MacAddress>, FieldError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse()
        .map(Some)
        .map_err(|error: MacAddressError| FieldError::new("macAddress", error.to_string()))
}

/// The device, when it can be the wake relay of `site`: it is at that site and has SSH on,
/// because the relay sends the Wake-on-LAN packet over its SSH link (D-7).
///
/// # Errors
/// A `deviceId` field error when there is no such device, it is at another site or its SSH is
/// off.
pub(in crate::devices) fn wake_relay(
    device: Option<&Device>,
    site: SiteId,
) -> Result<&Device, FieldError> {
    let reject = |message: &str| Err(FieldError::new("deviceId", message));
    match device {
        None => reject("There is no such device."),
        Some(device) if device.site != site => reject("Choose a device at this site."),
        Some(device) if !device.ssh.enabled => {
            reject("Turn on SSH for this device first: the relay wakes devices over SSH.")
        }
        Some(device) => Ok(device),
    }
}
