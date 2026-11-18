//! The alphabets and lengths of the texts in an `agentUsage` member (K8 Section 4). No text in
//! it is free text from the device: each passes one of these rules or is not sent.

use super::OTHER_MODEL;

/// The longest `state` or `source`, in bytes.
const MAX_WORD: usize = 32;

/// The longest session ID or model name, in bytes.
const MAX_NAME: usize = 64;

/// The longest repository name, in bytes.
const MAX_REPOSITORY: usize = 200;

/// The longest segment of a repository path, in bytes.
const MAX_SEGMENT: usize = 100;

/// The fewest and most path segments after the host of a repository.
const SEGMENTS: std::ops::RangeInclusive<usize> = 2..=8;

/// A `state` or `source`: 1 to 32 bytes of `[a-z_]`.
#[must_use]
pub fn is_word(text: &str) -> bool {
    sized(text, MAX_WORD)
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
}

/// A session ID: 1 to 64 bytes of `[0-9A-Za-z_-]`.
#[must_use]
pub fn is_session_id(text: &str) -> bool {
    sized(text, MAX_NAME)
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

/// A model name: 1 to 64 bytes of `[0-9A-Za-z._:-]`.
#[must_use]
pub fn is_model(text: &str) -> bool {
    sized(text, MAX_NAME)
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

/// `name` when it is a model name by [`is_model`], else [`OTHER_MODEL`].
#[must_use]
pub fn model_or_other(name: &str) -> &str {
    if is_model(name) { name } else { OTHER_MODEL }
}

/// A repository: at most 200 bytes, a host of `[a-z0-9.-]`, then 2 to 8 segments of 1 to 100
/// bytes of `[A-Za-z0-9._-]`, none `.` or `..`, each after one `/`.
#[must_use]
pub fn is_repository(text: &str) -> bool {
    let mut parts = text.split('/');
    let host = parts.next().unwrap_or_default();
    let host_fits = !host.is_empty()
        && host.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        });
    let segments: Vec<&str> = parts.collect();
    text.len() <= MAX_REPOSITORY
        && host_fits
        && SEGMENTS.contains(&segments.len())
        && segments.iter().all(|segment| is_segment(segment))
}

fn is_segment(text: &str) -> bool {
    sized(text, MAX_SEGMENT)
        && text != "."
        && text != ".."
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

const fn sized(text: &str, most: usize) -> bool {
    !text.is_empty() && text.len() <= most
}
