//! The forms of single values: a name of the file, an image, a whole number, a duration, a
//! size, a path in a container.

/// A name of a service, a volume, a network or a config: 1 to 63 characters of `a-z`, `0-9`,
/// `-` and `_`, and the first is a letter or a digit.
pub(crate) fn is_name(text: &str) -> bool {
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
pub(crate) fn is_image(text: &str) -> bool {
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
pub(crate) fn whole(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// A truth value as YAML writes it.
pub(crate) fn truth(text: &str) -> Option<bool> {
    match text {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// A duration of Compose in whole seconds: parts of a number and `h`, `m` or `s`
/// (`1m30s`), or a number of seconds alone. A part of a second is not permitted.
pub(crate) fn seconds(text: &str) -> Option<u64> {
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
pub(crate) fn bytes(text: &str) -> Option<u64> {
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

/// A count of processors above 0: a whole number, or one with a decimal part (`0.5`).
pub(crate) fn is_processors(text: &str) -> bool {
    let (units, part) = text.split_once('.').unwrap_or((text, "0"));
    match (whole(units), whole(part)) {
        (Some(units), Some(part)) => units > 0 || part > 0,
        _ => false,
    }
}

/// A path in a container that a mount may name: absolute, with no `..`, and not `/`.
pub(crate) fn is_target(text: &str) -> bool {
    text.starts_with('/')
        && text != "/"
        && !text.split('/').any(|part| part == "..")
        && !text.bytes().any(|byte| byte.is_ascii_control())
}

/// One line of text with no control character, for a value that Compose takes as it is.
pub(crate) fn is_line(text: &str) -> bool {
    !text.is_empty() && !text.bytes().any(|byte| byte.is_ascii_control())
}
