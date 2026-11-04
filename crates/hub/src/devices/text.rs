//! Rules for operator-typed text: the characters it may hold, and host names (DEV-010).
//!
//! Names, kinds, tags and user names reach the event log and the audit log, which are kept for
//! a year and cannot be edited (LOG-004).

use std::net::IpAddr;

use super::rules::FieldError;

const HOST_MAX: usize = 253;
const LABEL_MAX: usize = 63;

/// Whether the text has no control character or line or paragraph separator (each would split
/// or break a log line) and no character that changes the reading direction (it would disguise
/// the text).
pub(super) fn is_plain(text: &str) -> bool {
    !text.chars().any(|c| {
        c.is_control()
            || matches!(
                c,
                '\u{2028}'
                    | '\u{2029}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{061C}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2066}'..='\u{2069}'
            )
    })
}

/// An IP address or an RFC 1123 host name, without scheme, port or path (DEV-010).
pub(super) fn host(text: &str) -> Result<String, FieldError> {
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
