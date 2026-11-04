//! Secrets inside free text: event messages, device names and audit string values (LOG-007,
//! D-3 *Values*).

use super::{
    REDACTED,
    names::{Secrecy, secrecy},
};

/// Text that marks a whole string as a secret, compared case-sensitively.
const SECRET_MARKERS: [&str; 1] = ["PuTTY-User-Key-File-"];

/// Text that marks a whole string as a secret, compared ignoring ASCII case.
const SECRET_MARKERS_ANY_CASE: [&str; 2] = ["otpauth://", "authorization:"];

/// Words that introduce a credential (`Bearer eyJ…`, `token: abc…`), compared ignoring ASCII
/// case and followed by whitespace or `:`.
const SCHEMES: [&str; 3] = ["bearer", "basic", "token"];

/// The shortest text that counts as a credential after a scheme word or as a URL user.
const CREDENTIAL_MIN_LEN: usize = 8;

/// `"[redacted]"` if `text` holds a private key, an `otpauth://` URI or an `Authorization:`
/// header; otherwise `text` with every URL password, secret `key=value` or `key: value`, scheme
/// credential and JWT masked. Each pass takes time linear in the length of `text`.
pub(in crate::events) fn redact_text(text: &str) -> String {
    if is_secret_text(text) {
        return REDACTED.to_owned();
    }
    let masked = mask_url_userinfo(text);
    let masked = mask_key_values(&masked);
    let masked = mask_after_words(&masked, &SCHEMES, scheme_credential);
    mask_jwts(&masked)
}

fn is_secret_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (text.contains("-----BEGIN") && text.contains("PRIVATE KEY"))
        || SECRET_MARKERS.iter().any(|marker| text.contains(marker))
        || SECRET_MARKERS_ANY_CASE
            .iter()
            .any(|marker| lower.contains(marker))
}

/// A byte of base64, base64url or a JWT.
fn is_credential_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=' | b'-' | b'_' | b'.')
}

/// Credential-shaped: 8 or more credential bytes with a digit, one of `+/=_`, or a capital
/// after the first letter, so `connectivity`, `Required` and `deploy-user` are words, while
/// `dXNlcjpodW50ZXIy`, `ghp_AAAA…` and `eyJ…` are credentials.
fn looks_like_credential(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() >= CREDENTIAL_MIN_LEN
        && bytes.iter().copied().all(is_credential_byte)
        && bytes.iter().enumerate().any(|(index, byte)| {
            byte.is_ascii_digit()
                || matches!(byte, b'+' | b'/' | b'=' | b'_')
                || (index > 0 && byte.is_ascii_uppercase())
        })
}

/// Whether a word may start at byte `index`: at the start or after a byte that is not an ASCII
/// letter or digit.
fn starts_word(text: &str, index: usize) -> bool {
    index == 0 || !text.as_bytes()[index - 1].is_ascii_alphanumeric()
}

/// Replaces, after each of `words` that starts a word (ignoring ASCII case), the part of the
/// rest that `secret` finds: `secret(rest)` gives its byte range in `rest`.
fn mask_after_words(
    text: &str,
    words: &[&str],
    secret: impl Fn(&str) -> Option<(usize, usize)>,
) -> String {
    let lower = text.to_ascii_lowercase();
    let mut masked = String::with_capacity(text.len());
    let (mut copied, mut index) = (0, 0);
    while index < text.len() {
        let found = words
            .iter()
            .find(|word| lower[index..].starts_with(**word) && starts_word(text, index));
        let range = found.and_then(|word| {
            let after = index + word.len();
            secret(&text[after..]).map(|(start, end)| (after + start, after + end))
        });
        match range {
            Some((start, end)) => {
                masked.push_str(&text[copied..start]);
                masked.push_str(REDACTED);
                (copied, index) = (end, end);
            }
            None => index += text[index..].chars().next().map_or(1, char::len_utf8),
        }
    }
    masked.push_str(&text[copied..]);
    masked
}

/// A byte of a key in `key=value` text: ASCII letters, digits, `_`, `-` and `.`
/// (`spring.datasource.password`).
fn is_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
}

/// Masks the value of every `key=value` and `key: value` whose key names a secret by the same
/// rules as a JSON key (`password`, `PGPASSWORD`, `dbPassword`, `accessToken`, `api_key`,
/// `client_secret`, …), including embedded JSON (`{"password":"…"}`). The separator is `=` or
/// `:` with optional spaces around it and an optional quote before it; a quoted value is masked
/// inside its quotes. `true` and `false` are never masked, like JSON booleans.
fn mask_key_values(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let (mut copied, mut index) = (0, 0);
    while let Some(offset) = text[index..].find(['=', ':']) {
        let separator = index + offset;
        index = separator + 1;
        let secret_key =
            key_before(text, separator).is_some_and(|key| secrecy(key) != Secrecy::Plain);
        if let Some((start, end)) = secret_key.then(|| value_after(text, index)).flatten() {
            masked.push_str(&text[copied..start]);
            masked.push_str(REDACTED);
            (copied, index) = (end, end);
        }
    }
    masked.push_str(&text[copied..]);
    masked
}

/// The key that ends before the separator at `separator`: optional spaces, an optional quote,
/// then key bytes.
fn key_before(text: &str, separator: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut end = separator;
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    if end > 0 && matches!(bytes[end - 1], b'"' | b'\'') {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_key_byte(bytes[start - 1]) {
        start -= 1;
    }
    (start < end).then(|| &text[start..end])
}

/// The byte range of the value that starts at `from`: optional spaces, then the inside of a
/// quoted value, or an unquoted value up to whitespace, `&`, `;`, `,`, `}` or a quote (also
/// after an unterminated quote). An empty value, `true`, `false`, or an object or array (`{`,
/// `[`) gives `None`.
fn value_after(text: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut start = from;
    while start < bytes.len() && matches!(bytes[start], b' ' | b'\t') {
        start += 1;
    }
    // A found closing quote ends a masked value and the caller resumes after it; a search fails
    // only from the last quote of its kind. So the searches stay linear in the text's length.
    if let Some(&quote @ (b'"' | b'\'')) = bytes.get(start) {
        start += 1;
        if let Some(close) = bytes[start..].iter().position(|byte| *byte == quote) {
            return (close > 0).then_some((start, start + close));
        }
    }
    if matches!(bytes.get(start), Some(b'{' | b'[')) {
        return None;
    }
    let len = text[start..]
        .find(|c: char| c.is_whitespace() || matches!(c, '&' | ';' | ',' | '}' | '"' | '\''))
        .unwrap_or(text.len() - start);
    let value = &text[start..start + len];
    let boolean = value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false");
    (len > 0 && !boolean).then_some((start, start + len))
}

/// After a scheme word: whitespace or `:`, an optional quote, then a credential-shaped token.
fn scheme_credential(rest: &str) -> Option<(usize, usize)> {
    let gap = rest
        .bytes()
        .take_while(|byte| byte.is_ascii_whitespace() || *byte == b':')
        .count();
    if gap == 0 {
        return None;
    }
    let start = gap + usize::from(matches!(rest.as_bytes().get(gap), Some(b'"' | b'\'')));
    let len = rest.as_bytes()[start..]
        .iter()
        .take_while(|byte| is_credential_byte(**byte))
        .count();
    looks_like_credential(&rest[start..start + len]).then_some((start, start + len))
}

/// Replaces every JWT: `eyJ` starting a word, then three base64url segments split by `.`, the
/// payload not empty and the signature possibly empty (unsigned); a `.` right after it is kept.
fn mask_jwts(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let (mut copied, mut scanned_to) = (0, 0);
    for (start, _) in text.match_indices("eyJ") {
        // An `eyJ` inside a run already scanned is part of that run: skipping it keeps the
        // pass linear (`"eyJ.".repeat(…)`).
        if start < scanned_to || !starts_word(text, start) {
            continue;
        }
        let len = text[start..]
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            .count();
        scanned_to = start + len;
        let run = &text[start..start + len];
        let parts: Vec<&str> = run.split('.').collect();
        // `header.payload.` is an unsigned JWT; a fourth, empty part is a full stop after one.
        let jwt_len = match parts.as_slice() {
            [_, payload, _] if !payload.is_empty() => Some(run.len()),
            [_, payload, _, ""] if !payload.is_empty() => Some(run.len() - 1),
            _ => None,
        };
        if let Some(jwt_len) = jwt_len {
            masked.push_str(&text[copied..start]);
            masked.push_str(REDACTED);
            copied = start + jwt_len;
        }
    }
    masked.push_str(&text[copied..]);
    masked
}

/// Masks the userinfo secret of every `scheme://userinfo@host`. The authority ends at the first
/// `/`, `?`, `#` or whitespace after `://`, and the userinfo at the last `@` inside it. With a
/// `:`, the part after the first `:` is masked (`user:[redacted]@host`); without one, the
/// whole userinfo is masked when it is credential-shaped (`[redacted]@github.com`), so plain
/// users such as `root@host` stay. When the authority has no `@` but a `:` that does not start
/// a port, an unencoded `/`, `?` or `#` may sit in the password (`u:pa/ss@h`): it is masked up
/// to the next `@` in the same word.
fn mask_url_userinfo(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(scheme_end) = rest.find("://") {
        let (before, after) = rest.split_at(scheme_end + 3);
        masked.push_str(before);
        rest = match userinfo_secret(after) {
            Some((start, end)) => {
                masked.push_str(&after[..start]);
                masked.push_str(REDACTED);
                &after[end..]
            }
            None => after,
        };
    }
    masked.push_str(rest);
    masked
}

/// The byte range of the secret in the userinfo at the start of `url` (the text after `://`).
/// It reads only the authority and, for an unencoded password, the text up to the next `@`,
/// whitespace or `://`, so the scan over all URLs stays linear (`"://".repeat(…)`).
fn userinfo_secret(url: &str) -> Option<(usize, usize)> {
    let authority_len = url
        .find(|c: char| matches!(c, '/' | '?' | '#') || c.is_whitespace())
        .unwrap_or(url.len());
    let authority = &url[..authority_len];
    if let Some(at) = authority.rfind('@') {
        let userinfo = &authority[..at];
        return match userinfo.find(':') {
            Some(colon) => Some((colon + 1, at)),
            None => looks_like_credential(userinfo).then_some((0, at)),
        };
    }
    let host_start = if authority.starts_with('[') {
        authority.find(']')?
    } else {
        0
    };
    let colon = host_start + authority[host_start..].find(':')?;
    let port = &authority[colon + 1..];
    if !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let tail = &url[authority_len..];
    let at = tail
        .char_indices()
        .take_while(|(index, c)| !c.is_whitespace() && !tail[*index..].starts_with("://"))
        .find(|(_, c)| *c == '@')?
        .0;
    Some((colon + 1, authority_len + at))
}
