//! The answers of port 80 (Section 12.2, SF-NEW-routeheaders): the ACME `HTTP-01` challenge of a
//! name and a redirect to `https://`. Port 80 never carries the content of an application. Pure.

use std::{collections::HashMap, hash::BuildHasher};

/// The path of an ACME challenge.
const CHALLENGE: &str = "/.well-known/acme-challenge/";

/// The answer to a request on port 80 for `host` and `target`. `names` holds the names that
/// serve; `challenges` maps each open token to its key authorization.
#[must_use]
pub fn answer<S: BuildHasher>(
    host: &str,
    target: &str,
    names: &[String],
    challenges: &HashMap<String, String, S>,
) -> Vec<u8> {
    let host = host
        .strip_suffix(":80")
        .unwrap_or(host)
        .to_ascii_lowercase();
    if let Some(token) = target.strip_prefix(CHALLENGE)
        && let Some(authorization) = challenges.get(token)
    {
        return format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\n\
             connection: close\r\n\r\n{authorization}",
            authorization.len()
        )
        .into_bytes();
    }
    if names.contains(&host) && target.starts_with('/') && !target.contains(['\r', '\n']) {
        return format!(
            "HTTP/1.1 308 Permanent Redirect\r\nlocation: https://{host}{target}\r\n\
             content-length: 0\r\nconnection: close\r\n\r\n"
        )
        .into_bytes();
    }
    b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".to_vec()
}

/// The `Host` and the target of a plain request head; `None` when it cannot be read.
#[must_use]
pub fn host_and_target(head: &[u8]) -> Option<(String, String)> {
    let text = std::str::from_utf8(head).ok()?;
    let mut lines = text.split("\r\n");
    let target = lines.next()?.split(' ').nth(1)?.to_owned();
    let host = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("host")
            .then(|| value.trim().to_owned())
    })?;
    Some((host, target))
}
