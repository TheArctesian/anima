//! What a web interface may be: the URL, label and fingerprint rules, and the origin
//! comparison. Pure; no I/O (WEB-001, WEB-008, D-4 to D-6).

use reqwest::Url;

use super::model::{FieldError, Fingerprint, OpenIn, WebInterfaceInput};

const URL_MAX_BYTES: usize = 2048;
const LABEL_MAX_CHARS: usize = 100;
const FINGERPRINT_DIGITS: usize = 64;

/// An input that passed the rules: the URL trimmed, the label filled in.
pub(super) struct NewInterface {
    pub label: String,
    pub url: String,
    pub open_in: OpenIn,
}

/// Checks an interface to create or to replace one with; every field in error, all at once.
pub(super) fn validate(input: &WebInterfaceInput) -> Result<NewInterface, Vec<FieldError>> {
    let parsed = parse_url(&input.url);
    let label = parsed.as_ref().map_or_else(
        |_| label(input.label.as_deref(), None),
        |(_, url)| label(input.label.as_deref(), url.host_str()),
    );
    match (parsed, label) {
        (Ok((url, _)), Ok(label)) => Ok(NewInterface {
            label,
            url,
            open_in: input.open_in,
        }),
        (url, label) => Err([url.err(), label.err()].into_iter().flatten().collect()),
    }
}

/// The trimmed text and its parse, when it is a plain `http` or `https` address (D-4).
pub(super) fn parse_url(text: &str) -> Result<(String, Url), FieldError> {
    let text = text.trim();
    let invalid = |message: &str| FieldError::new("url", message);
    if text.len() > URL_MAX_BYTES {
        return Err(invalid("The URL is longer than 2048 bytes."));
    }
    if text
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || is_invisible(c))
    {
        return Err(invalid(
            "The URL cannot contain spaces, control or invisible characters.",
        ));
    }
    let url = Url::parse(text).map_err(|_| invalid("Enter a full web address."))?;
    let lower = text.to_ascii_lowercase();
    let after_scheme = lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"));
    // The parser would read `https:///x` as host `x` and `\` as `/`; the operator typed neither.
    let authority_has_at = |rest: &str| {
        rest.split(['/', '?'])
            .next()
            .is_some_and(|a| a.contains('@'))
    };
    if after_scheme
        .is_none_or(|rest| rest.starts_with('/') || rest.contains('\\') || authority_has_at(rest))
    {
        return Err(invalid("The URL must start with http:// or https://."));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(invalid("The URL needs a host."));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("The URL cannot contain a user name or password."));
    }
    if url.fragment().is_some() {
        return Err(invalid("The URL cannot contain a fragment."));
    }
    Ok((text.to_owned(), url))
}

/// The typed label trimmed, or the host when it is empty or missing.
fn label(typed: Option<&str>, host: Option<&str>) -> Result<String, FieldError> {
    let typed = typed.map(str::trim).filter(|text| !text.is_empty());
    let Some(text) = typed else {
        return Ok(host.map_or_else(String::new, host_label));
    };
    if text.chars().any(is_unsafe_in_label) {
        Err(FieldError::new(
            "label",
            "The label cannot contain control characters or text-direction marks.",
        ))
    } else if text.chars().count() > LABEL_MAX_CHARS {
        Err(FieldError::new(
            "label",
            format!("The label must be at most {LABEL_MAX_CHARS} characters."),
        ))
    } else {
        Ok(text.to_owned())
    }
}

/// The label an interface gets from its host.
pub(super) fn host_label(host: &str) -> String {
    host.chars().take(LABEL_MAX_CHARS).collect()
}

fn is_unsafe_in_label(c: char) -> bool {
    c.is_control() || is_invisible(c)
}

/// Format characters (Unicode Cf) that show nothing or reorder text: zero-width marks,
/// text-direction controls, joiners, the byte order mark and tag characters.
fn is_invisible(c: char) -> bool {
    matches!(c,
        '\u{ad}' | '\u{600}'..='\u{605}' | '\u{61c}' | '\u{6dd}' | '\u{70f}' | '\u{180e}'
        | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{206f}' | '\u{feff}' | '\u{fff9}'..='\u{fffb}'
        | '\u{e0000}'..='\u{e007f}')
}

/// The fingerprint in 64 hex digits, colons between byte pairs allowed, any case (D-5).
pub(super) fn fingerprint(text: &str) -> Result<Fingerprint, FieldError> {
    let digits: String = if text.contains(':') {
        let pairs: Vec<&str> = text.split(':').collect();
        if pairs.len() * 2 != FINGERPRINT_DIGITS || pairs.iter().any(|pair| pair.len() != 2) {
            String::new()
        } else {
            pairs.concat()
        }
    } else {
        text.to_owned()
    };
    if digits.len() == FINGERPRINT_DIGITS && digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(Fingerprint(digits.to_ascii_lowercase()))
    } else {
        Err(FieldError::new(
            "fingerprintSha256",
            "Enter 64 hexadecimal digits, with or without colons between byte pairs.",
        ))
    }
}

/// Whether both addresses have the same scheme, host and port (D-6). An address that does
/// not parse has no origin, so it matches nothing.
pub(super) fn same_origin(a: &str, b: &str) -> bool {
    match (Url::parse(a), Url::parse(b)) {
        (Ok(a), Ok(b)) => a.origin() == b.origin(),
        _ => false,
    }
}

/// Whether the address is `https`.
pub(super) fn is_https(text: &str) -> bool {
    Url::parse(text).is_ok_and(|url| url.scheme() == "https")
}
