//! The two changes of the text: one member set, one member taken out (MCP-030).

use super::{JsoncError, Object, Scan, object_at, only};

/// The line break that the file uses.
fn line_break(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// The white space at the start of the line that holds `at`.
fn indent_of(text: &str, at: usize) -> &str {
    let line = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
    let end = text[line..]
        .find(|character: char| character != ' ' && character != '\t')
        .map_or(text.len(), |end| line + end);
    &text[line..end.min(at)]
}

/// `value` with each line after the first moved in by `indent`, with the line break `eol`.
fn indented(value: &str, indent: &str, eol: &str) -> String {
    value.replace('\n', &format!("{eol}{indent}"))
}

/// `"name": value` put into `object` as its last member.
fn insert(text: &str, object: &Object, name: &str, value: &str) -> String {
    let key = serde_json::Value::from(name).to_string();
    let eol = line_break(text);
    match object.members.last() {
        None => {
            let outer = indent_of(text, object.open);
            let inner = format!("{outer}  ");
            // What is between the braces stays: it can hold a comment.
            let inside = &text[object.open + 1..object.close];
            let kept = if inside.trim().is_empty() {
                ""
            } else {
                inside.trim_end_matches([' ', '\t', '\r', '\n'])
            };
            format!(
                "{}{kept}{eol}{inner}{key}: {}{eol}{outer}{}",
                &text[..=object.open],
                indented(value, &inner, eol),
                &text[object.close..]
            )
        }
        Some(last) => {
            let inner = indent_of(text, last.span.start);
            let member = format!("{eol}{inner}{key}: {}", indented(value, inner, eol));
            match last.comma {
                // The file ends its members with a comma, so the new member does also.
                Some(comma) => format!("{}{member},{}", &text[..=comma], &text[comma + 1..]),
                None => format!(
                    "{},{member}{}",
                    &text[..last.span.end],
                    &text[last.span.end..]
                ),
            }
        }
    }
}

/// The text with `name` in the object at `path` set to `value` (JSON text, as `serde_json` prints
/// it with two spaces). An object of the path that is not there is made. Only that member changes.
///
/// # Errors
/// [`JsoncError`] when the text is not an object, cannot be read, holds a different kind of
/// value at the path, or holds a key of the path or `name` twice.
pub fn set(text: &str, path: &[&str], name: &str, value: &str) -> Result<String, JsoncError> {
    let text = if text.trim().is_empty() { "{}\n" } else { text };
    let scan = Scan {
        bytes: text.as_bytes(),
    };
    let eol = line_break(text);
    // The deepest object of the path that is there; the rest of the path is made inside the value.
    for depth in (0..=path.len()).rev() {
        let Some(object) = object_at(&scan, &path[..depth])? else {
            continue;
        };
        if depth == path.len() {
            if let Some(found) = only(&object, name)? {
                let inner = indent_of(text, found.span.start);
                return Ok(format!(
                    "{}{}{}",
                    &text[..found.value.start],
                    indented(value, inner, eol),
                    &text[found.value.end..]
                ));
            }
            return Ok(insert(text, &object, name, value));
        }
        let mut nested = format!(
            "{{\n  {}: {}\n}}",
            serde_json::Value::from(name),
            indented(value, "  ", "\n")
        );
        for key in path[depth + 1..].iter().rev() {
            nested = format!(
                "{{\n  {}: {}\n}}",
                serde_json::Value::from(*key),
                indented(&nested, "  ", "\n")
            );
        }
        return Ok(insert(text, &object, path[depth], &nested));
    }
    Err(JsoncError::NotAnObject)
}

/// Whether `text` holds only spaces, tabs and line breaks.
fn blank(text: &str) -> bool {
    text.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
}

/// The index of the line break before `at` when only spaces are between them, else `at`.
fn line_start(text: &str, at: usize) -> usize {
    let before = text[..at].trim_end_matches([' ', '\t']);
    match before.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').map_or(line.len(), str::len),
        None => at,
    }
}

/// `text` without the bytes of `cuts`, which are in order and do not overlap.
fn without(text: &str, cuts: &[(usize, usize)]) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut at = 0;
    for cut in cuts {
        kept.push_str(&text[at..cut.0]);
        at = cut.1;
    }
    kept.push_str(&text[at..]);
    kept
}

/// The text with `name` taken out of the object at `path`, or `None` when it is not there. The
/// objects of the path stay, also when they are then empty. A comment beside the member stays.
///
/// # Errors
/// [`JsoncError`] when the text is not an object, cannot be read, or holds a key twice.
pub fn remove(text: &str, path: &[&str], name: &str) -> Result<Option<String>, JsoncError> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let scan = Scan {
        bytes: text.as_bytes(),
    };
    let Some(object) = object_at(&scan, path)? else {
        return Ok(None);
    };
    let Some(found) = only(&object, name)? else {
        return Ok(None);
    };
    let at = object
        .members
        .iter()
        .position(|member| member.span == found.span)
        .unwrap_or_default();
    let before = at.checked_sub(1).map(|before| &object.members[before]);
    let place = found.span.clone();
    let cuts = match (found.comma, before) {
        (Some(comma), _) => {
            // The member goes with its comma. It takes the line break before it only when it has
            // its line to itself: a member that follows on the same line must not join a comment.
            let rest = text[comma + 1..].split('\n').next().unwrap_or_default();
            let start = if blank(rest) {
                line_start(text, place.start)
            } else {
                place.start
            };
            if blank(&text[place.end..comma]) {
                vec![(start, comma + 1)]
            } else {
                vec![(start, place.end), (comma, comma + 1)]
            }
        }
        // The last member of several: the member before it loses its comma.
        (None, Some(before)) => match before.comma {
            Some(comma) if !blank(&text[comma + 1..place.start]) => {
                vec![
                    (comma, comma + 1),
                    (line_start(text, place.start), place.end),
                ]
            }
            _ => vec![(before.span.end, place.end)],
        },
        // The only member: the object is then empty, but for a comment that it holds.
        (None, None)
            if blank(&text[object.open + 1..place.start])
                && blank(&text[place.end..object.close]) =>
        {
            vec![(object.open + 1, object.close)]
        }
        (None, None) => vec![(line_start(text, place.start), place.end)],
    };
    Ok(Some(without(text, &cuts)))
}
