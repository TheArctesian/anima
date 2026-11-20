//! The forms of the single values of a request and of the settings: an identifier, a name, a
//! path, an origin. Each value that goes into a list of arguments has one of these forms.

/// `length` lower-case hexadecimal characters.
pub(crate) fn is_lower_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A UUID in the form of 36 lower-case characters (K10 Section 6.4.6).
pub(crate) fn is_uuid(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let lengths = [8, 4, 4, 4, 12];
    parts.len() == lengths.len()
        && parts
            .iter()
            .zip(lengths)
            .all(|(part, length)| is_lower_hex(part, length))
}

/// The digest of an image as the engine writes it: `sha256:` and 64 hexadecimal characters.
pub(crate) fn is_digest(text: &str) -> bool {
    text.strip_prefix("sha256:")
        .is_some_and(|hex| is_lower_hex(hex, 64))
}

/// The name of a Compose project: 1 to 63 characters of `a-z`, `0-9`, `-` and `_`, and the
/// first is a letter or a digit. The contract gives no form for the name of an adopted
/// project; this is the form that Compose itself permits, with the length of a name of
/// Section 6.4.4.
pub(crate) fn is_project_name(text: &str) -> bool {
    let first = text
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    first
        && text.len() <= 63
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte))
}

/// A name of a container, a network or a volume as the engine permits it: a letter or a digit,
/// then letters, digits, `_`, `.` and `-`; 255 characters at most. It can never be read as an
/// option of a command.
pub(crate) fn is_engine_name(text: &str) -> bool {
    let first = text
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    first
        && text.len() <= 255
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
}

/// An absolute path of the machine with no `.` part, no `..` part, no empty part and no
/// control character; 4,096 bytes at most. An address of a different machine has a scheme and
/// is thus never such a path.
pub(crate) fn is_path(text: &str) -> bool {
    let Some(below) = text.strip_prefix('/') else {
        return false;
    };
    text.len() <= 4096
        && !below.is_empty()
        && !text.bytes().any(|byte| byte.is_ascii_control())
        && below
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The host of an origin: a name or an IPv4 address in lower case, or an IPv6 address in
/// brackets.
fn is_host(text: &str) -> bool {
    if let Some(inner) = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return inner.parse::<std::net::Ipv6Addr>().is_ok()
            && !inner.bytes().any(|byte| byte.is_ascii_uppercase());
    }
    let sound =
        |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte);
    (1..=253).contains(&text.len())
        && text.bytes().all(sound)
        && !text.starts_with(['.', '-'])
        && !text.ends_with(['.', '-'])
}

/// The origin of a hub: the scheme, the host in lower case and the port, always written
/// (K10 Section 6.4.2), as `https://hub.example:443`.
pub(crate) fn is_origin(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once("://") else {
        return false;
    };
    let Some((host, port)) = rest.rsplit_once(':') else {
        return false;
    };
    let port_sound = !port.starts_with('0')
        && port.len() <= 5
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok();
    matches!(scheme, "http" | "https") && is_host(host) && port_sound
}

/// The name of the project of an application that Anima installed: `anima-` and the 32
/// hexadecimal digits of its UUID (K10 Section 6.4.6).
pub(crate) fn project_of(application: &str) -> String {
    format!("anima-{}", application.replace('-', ""))
}
