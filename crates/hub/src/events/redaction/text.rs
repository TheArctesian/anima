//! Secrets inside free text: event messages, device names and audit string values (LOG-007,
//! D-3 *Values*).

use super::REDACTED;

/// Text that marks a whole string as a secret, compared case-sensitively.
const SECRET_MARKERS: [&str; 1] = ["PuTTY-User-Key-File-"];

/// Text that marks a whole string as a secret, compared ignoring ASCII case.
const SECRET_MARKERS_ANY_CASE: [&str; 2] = ["otpauth://", "authorization:"];

/// Words that introduce a credential (`Bearer eyJ…`, `token: abc…`), compared ignoring ASCII
/// case and followed by whitespace or `:`.
const SCHEMES: [&str; 3] = ["bearer", "basic", "token"];

/// Keys of `key=value` pairs whose value is a secret (libpq `password=…`, JDBC `?pwd=…`),
/// compared ignoring ASCII case.
const SECRET_KEYS: [&str; 4] = ["password", "passwd", "pwd", "token"];

/// The shortest text that counts as a credential after a scheme word or as a URL user.
const CREDENTIAL_MIN_LEN: usize = 8;

/// `"[redacted]"` if `text` holds a private key, an `otpauth://` URI or an `Authorization:`
/// header; otherwise `text` with every URL password, secret `key=value`, scheme credential and
/// JWT masked.
pub(in crate::events) fn redact_text(text: &str) -> String {
    if is_secret_text(text) {
        return REDACTED.to_owned();
    }
    let masked = mask_url_userinfo(text);
    let masked = mask_after_words(&masked, &SECRET_KEYS, key_value);
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

/// After a secret key: `=` and a value up to whitespace, `&`, `;` or `,`, or the inside of a
/// quoted value.
fn key_value(rest: &str) -> Option<(usize, usize)> {
    let value = rest.strip_prefix('=')?;
    let len = match value.as_bytes().first() {
        Some(&quote @ (b'\'' | b'"')) => {
            let close = value[1..].find(char::from(quote))?;
            return (close > 0).then_some((2, 2 + close));
        }
        _ => value
            .find(|c: char| c.is_whitespace() || matches!(c, '&' | ';' | ','))
            .unwrap_or(value.len()),
    };
    (len > 0).then_some((1, 1 + len))
}

/// After a scheme word: whitespace or `:`, then a credential-shaped token.
fn scheme_credential(rest: &str) -> Option<(usize, usize)> {
    let gap = rest
        .bytes()
        .take_while(|byte| byte.is_ascii_whitespace() || *byte == b':')
        .count();
    let len = rest.as_bytes()[gap..]
        .iter()
        .take_while(|byte| is_credential_byte(**byte))
        .count();
    (gap > 0 && looks_like_credential(&rest[gap..gap + len])).then_some((gap, gap + len))
}

/// Replaces every JWT: `eyJ` starting a word, then three base64url segments split by `.`, the
/// payload not empty and the signature possibly empty (unsigned); a `.` right after it is kept.
fn mask_jwts(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut copied = 0;
    for (start, _) in text.match_indices("eyJ") {
        if start < copied || !starts_word(text, start) {
            continue;
        }
        let len = text[start..]
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            .count();
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
fn userinfo_secret(url: &str) -> Option<(usize, usize)> {
    let word = &url[..url.find(char::is_whitespace).unwrap_or(url.len())];
    let authority = &word[..word.find(['/', '?', '#']).unwrap_or(word.len())];
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
    let at = word[authority.len()..].find('@')? + authority.len();
    Some((colon + 1, at))
}
