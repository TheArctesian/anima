//! Pure validation rules for check configuration. Errors name the field, never its value.

/// Which field of a check configuration was refused, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {reason}")]
pub struct ConfigError {
    pub field: &'static str,
    pub reason: Reason,
}

/// Why a field was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Reason {
    #[error("must be between {min} and {max}")]
    OutOfRange { min: u32, max: u32 },
    #[error("must not be empty")]
    Empty,
    #[error("is longer than {max} bytes")]
    TooLong { max: usize },
    #[error("has more than {max} entries")]
    TooMany { max: usize },
    #[error("is not valid")]
    Invalid,
    #[error("must use http or https")]
    Scheme,
    #[error("must not contain a user name or password")]
    Userinfo,
    #[error("must not contain control characters")]
    ControlCharacter,
    #[error("needs a value that is missing")]
    Missing,
}

pub const TIMEOUT_MS: (u32, u32) = (100, 60_000);
pub const URL_MAX: usize = 2048;
pub const HEADERS_MAX: usize = 20;
pub const BODY_MAX: usize = 64 * 1024;
pub const KEYWORD_MAX: usize = 1024;
pub const JSON_QUERY_MAX: usize = 512;
pub const DNS_NAME_MAX: usize = 253;
pub const REDIRECTS_MAX: u8 = 10;

pub(super) const fn err(field: &'static str, reason: Reason) -> ConfigError {
    ConfigError { field, reason }
}

pub(super) fn timeout(field: &'static str, ms: u32) -> Result<(), ConfigError> {
    let (min, max) = TIMEOUT_MS;
    if (min..=max).contains(&ms) {
        Ok(())
    } else {
        Err(err(field, Reason::OutOfRange { min, max }))
    }
}

pub(super) fn port(field: &'static str, port: u16) -> Result<(), ConfigError> {
    if port == 0 {
        Err(err(
            field,
            Reason::OutOfRange {
                min: 1,
                max: 65_535,
            },
        ))
    } else {
        Ok(())
    }
}

pub(super) fn max_len(field: &'static str, text: &str, max: usize) -> Result<(), ConfigError> {
    if text.len() > max {
        Err(err(field, Reason::TooLong { max }))
    } else {
        Ok(())
    }
}

/// A header value: no CR, LF or NUL (header injection).
pub(super) fn header_value(field: &'static str, value: &str) -> Result<(), ConfigError> {
    if value.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0)) {
        Err(err(field, Reason::ControlCharacter))
    } else {
        Ok(())
    }
}

/// RFC 9110 token: the grammar of a header name.
pub(super) fn header_name(field: &'static str, name: &str) -> Result<(), ConfigError> {
    let token = |b: u8| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b);
    if name.is_empty() {
        Err(err(field, Reason::Empty))
    } else if name.len() > 128 {
        Err(err(field, Reason::TooLong { max: 128 }))
    } else if name.bytes().all(token) {
        Ok(())
    } else {
        Err(err(field, Reason::Invalid))
    }
}

/// Header names whose values are secrets (compared case-insensitively).
#[must_use]
pub fn is_secret_header(name: &str) -> bool {
    ["authorization", "proxy-authorization", "cookie"]
        .iter()
        .any(|secret| name.eq_ignore_ascii_case(secret))
}

/// A DNS name: dot-separated labels of 1..=63 letters, digits and hyphens (and underscores
/// when `underscore` is set, as service records use them); a trailing dot is allowed.
pub(super) fn dns_name(
    field: &'static str,
    name: &str,
    underscore: bool,
) -> Result<(), ConfigError> {
    let bare = name.strip_suffix('.').unwrap_or(name);
    if bare.is_empty() {
        return Err(err(field, Reason::Empty));
    }
    if bare.len() > DNS_NAME_MAX {
        return Err(err(field, Reason::TooLong { max: DNS_NAME_MAX }));
    }
    let label_ok = |label: &str| {
        (1..=63).contains(&label.len())
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || (underscore && b == b'_'))
    };
    if bare.split('.').all(label_ok) {
        Ok(())
    } else {
        Err(err(field, Reason::Invalid))
    }
}

/// RFC 6901 JSON Pointer: empty, or `/` segments where `~` is followed by `0` or `1`.
pub(super) fn json_pointer(field: &'static str, pointer: &str) -> Result<(), ConfigError> {
    max_len(field, pointer, JSON_QUERY_MAX)?;
    if !pointer.is_empty() && !pointer.starts_with('/') {
        return Err(err(field, Reason::Invalid));
    }
    let mut bytes = pointer.bytes().peekable();
    while let Some(b) = bytes.next() {
        if b == b'~' && !matches!(bytes.peek(), Some(b'0' | b'1')) {
            return Err(err(field, Reason::Invalid));
        }
    }
    Ok(())
}
