//! The forms of single values: a name of the file, an image, a whole number, a duration, a
//! size, a path in a container. The applications program reads each value of the final form
//! with these readers, so that the check and the final form read one value in one way.

/// A name of a service, a volume, a network or a config: 1 to 63 characters of `a-z`, `0-9`,
/// `-` and `_`, and the first is a letter or a digit.
#[must_use]
pub fn is_name(text: &str) -> bool {
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

/// An image reference: it starts with `a-z` or `0-9`, has only the characters that a registry
/// permits, and names a tag or a digest. It can thus never be read as an option of a command.
#[must_use]
pub fn is_image(text: &str) -> bool {
    let first = text
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    let last_part = text.rsplit('/').next().unwrap_or_default();
    first
        && text.len() <= 512
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-/:@".contains(&byte))
        && (last_part.contains(':') || last_part.contains('@'))
        && !text.ends_with(':')
        && !text.ends_with('@')
}

/// A whole number of 0 or more, with no sign and no other character.
#[must_use]
pub fn whole(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// A truth value as YAML writes it.
#[must_use]
pub fn truth(text: &str) -> Option<bool> {
    match text {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// A duration of Compose in whole seconds: parts of a number and `h`, `m` or `s`
/// (`1m30s`), or a number of seconds alone. A part of a second is not permitted.
#[must_use]
pub fn seconds(text: &str) -> Option<u64> {
    if let Some(plain) = whole(text) {
        return Some(plain);
    }
    let mut total: u64 = 0;
    let mut digits = String::new();
    for character in text.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
            continue;
        }
        let unit = match character {
            'h' => 3600,
            'm' => 60,
            's' => 1,
            _ => return None,
        };
        total = total.checked_add(whole(&digits)?.checked_mul(unit)?)?;
        digits.clear();
    }
    digits.is_empty().then_some(total)
}

/// A size of Compose in bytes: a whole number, or one with `b`, `k`, `m` or `g` (also `kb`,
/// `mb`, `gb`).
#[must_use]
pub fn bytes(text: &str) -> Option<u64> {
    let lower = text.to_ascii_lowercase();
    let digits_end = lower
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(lower.len());
    let (digits, unit) = lower.split_at(digits_end);
    let scale: u64 = match unit {
        "" | "b" => 1,
        "k" | "kb" => 1 << 10,
        "m" | "mb" => 1 << 20,
        "g" | "gb" => 1 << 30,
        _ => return None,
    };
    whole(digits)?.checked_mul(scale)
}

/// A count of processors in billionths of one processor, as the engine counts them. A digit
/// after the ninth of the decimal part is dropped.
#[must_use]
pub fn processor_billionths(text: &str) -> Option<u64> {
    let (units, part) = text.split_once('.').unwrap_or((text, "0"));
    whole(part)?;
    let part: String = part.chars().chain("000000000".chars()).take(9).collect();
    whole(units)?
        .checked_mul(1_000_000_000)?
        .checked_add(whole(&part)?)
}

/// The name of the folder of a bind mount: the source is exactly `binds/<name>` of the
/// application, with or without `./` before it (K10 Section 6.4.5). The name has the form of
/// a name of the file, which is inside what Section 6.4.5 permits.
#[must_use]
pub fn bind_name(source: &str) -> Option<&str> {
    let relative = source.strip_prefix("./").unwrap_or(source);
    relative.strip_prefix("binds/").filter(|name| is_name(name))
}

/// A count of processors above 0: a whole number, or one with a decimal part (`0.5`).
#[must_use]
pub fn is_processors(text: &str) -> bool {
    let (units, part) = text.split_once('.').unwrap_or((text, "0"));
    match (whole(units), whole(part)) {
        (Some(units), Some(part)) => units > 0 || part > 0,
        _ => false,
    }
}

/// A path in a container that a mount may name: absolute, not `/`, and each part after a `/`
/// is a name other than `.` and `..`. Thus it has no `//` and no `/` at its end, and two
/// texts that are different are two different paths.
#[must_use]
pub fn is_target(text: &str) -> bool {
    let Some(below) = text.strip_prefix('/') else {
        return false;
    };
    below
        .split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..")
        && !text.bytes().any(|byte| byte.is_ascii_control())
}

/// One line of text with no control character, for a value that Compose takes as it is.
#[must_use]
pub fn is_line(text: &str) -> bool {
    !text.is_empty() && !text.bytes().any(|byte| byte.is_ascii_control())
}
