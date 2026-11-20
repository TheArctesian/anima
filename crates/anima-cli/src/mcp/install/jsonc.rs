//! One member of a JSON object, set or removed in the text of a file, with every other byte kept
//! (MCP-030). The files of other programs hold comments, trailing commas and an order that a
//! parser and a writer would lose, so this module only finds the place and cuts the text there.

use std::ops::Range;

/// Why the text was not changed. No variant carries text of the file (LOG-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum JsoncError {
    #[error("it is not JSON that this program can read")]
    Malformed,
    #[error("its top level is not an object")]
    NotAnObject,
    #[error("the place of the servers holds a value that is not an object")]
    PlaceNotAnObject,
    #[error("it is nested deeper than this program reads")]
    TooDeep,
}

/// The deepest nesting that is read: a hostile file cannot make the reader use the whole stack.
const DEPTH_LIMIT: usize = 64;

struct Member {
    key: String,
    /// From the opening quote of the key to the end of the value.
    span: Range<usize>,
    value: Range<usize>,
    /// The index of the comma after the value, when there is one.
    comma: Option<usize>,
}

struct Object {
    /// The index of `{` and of `}`.
    open: usize,
    close: usize,
    members: Vec<Member>,
}

struct Scan<'a> {
    bytes: &'a [u8],
}

impl Scan<'_> {
    /// The index of the first byte at or after `at` that is not space and not a comment.
    fn trivia(&self, mut at: usize) -> Result<usize, JsoncError> {
        loop {
            match (self.bytes.get(at), self.bytes.get(at + 1)) {
                (Some(b' ' | b'\t' | b'\n' | b'\r'), _) => at += 1,
                (Some(b'/'), Some(b'/')) => {
                    at = self.bytes[at..].iter().position(|byte| *byte == b'\n').map_or(self.bytes.len(), |end| at + end);
                }
                (Some(b'/'), Some(b'*')) => {
                    let end = self.bytes[at + 2..].windows(2).position(|pair| pair == b"*/").ok_or(JsoncError::Malformed)?;
                    at += end + 4;
                }
                _ => return Ok(at),
            }
        }
    }

    /// The index after the string that starts at `at`.
    fn string(&self, at: usize) -> Result<usize, JsoncError> {
        let mut at = at + 1;
        loop {
            match self.bytes.get(at).ok_or(JsoncError::Malformed)? {
                b'"' => return Ok(at + 1),
                b'\\' => at += 2,
                b'\n' => return Err(JsoncError::Malformed),
                _ => at += 1,
            }
        }
    }

    /// The index after the value that starts at `at`.
    fn value(&self, at: usize, depth: usize) -> Result<usize, JsoncError> {
        match self.bytes.get(at).ok_or(JsoncError::Malformed)? {
            b'"' => self.string(at),
            b'{' => Ok(self.object(at, depth)?.close + 1),
            b'[' => self.array(at, depth),
            _ => {
                let end = self.bytes[at..]
                    .iter()
                    .position(|byte| matches!(byte, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r' | b'/'))
                    .map_or(self.bytes.len(), |end| at + end);
                if end == at { Err(JsoncError::Malformed) } else { Ok(end) }
            }
        }
    }

    fn array(&self, at: usize, depth: usize) -> Result<usize, JsoncError> {
        if depth >= DEPTH_LIMIT {
            return Err(JsoncError::TooDeep);
        }
        let mut at = self.trivia(at + 1)?;
        loop {
            if self.bytes.get(at) == Some(&b']') {
                return Ok(at + 1);
            }
            at = self.trivia(self.value(at, depth + 1)?)?;
            match self.bytes.get(at) {
                Some(b',') => at = self.trivia(at + 1)?,
                Some(b']') => {}
                _ => return Err(JsoncError::Malformed),
            }
        }
    }

    fn object(&self, open: usize, depth: usize) -> Result<Object, JsoncError> {
        if depth >= DEPTH_LIMIT {
            return Err(JsoncError::TooDeep);
        }
        let mut members = Vec::new();
        let mut at = self.trivia(open + 1)?;
        loop {
            match self.bytes.get(at) {
                Some(b'}') => return Ok(Object { open, close: at, members }),
                Some(b'"') => {}
                _ => return Err(JsoncError::Malformed),
            }
            let key_end = self.string(at)?;
            let key = std::str::from_utf8(&self.bytes[at..key_end]).ok().and_then(|text| serde_json::from_str(text).ok()).ok_or(JsoncError::Malformed)?;
            let colon = self.trivia(key_end)?;
            if self.bytes.get(colon) != Some(&b':') {
                return Err(JsoncError::Malformed);
            }
            let start = self.trivia(colon + 1)?;
            let end = self.value(start, depth + 1)?;
            let after = self.trivia(end)?;
            let comma = (self.bytes.get(after) == Some(&b',')).then_some(after);
            members.push(Member { key, span: at..end, value: start..end, comma });
            at = match comma {
                Some(comma) => self.trivia(comma + 1)?,
                None => after,
            };
        }
    }
}

/// The object at `path` below the top level, or `None` when a key of the path is not there.
fn object_at(scan: &Scan<'_>, path: &[&str]) -> Result<Option<Object>, JsoncError> {
    let start = scan.trivia(0)?;
    if scan.bytes.get(start) != Some(&b'{') {
        return Err(JsoncError::NotAnObject);
    }
    let mut object = scan.object(start, 0)?;
    if scan.trivia(object.close + 1)? != scan.bytes.len() {
        return Err(JsoncError::Malformed);
    }
    for key in path {
        let Some(member) = object.members.iter().find(|member| member.key == *key) else { return Ok(None) };
        if scan.bytes.get(member.value.start) != Some(&b'{') {
            return Err(JsoncError::PlaceNotAnObject);
        }
        object = scan.object(member.value.start, 0)?;
    }
    Ok(Some(object))
}

/// The text of the value of `name` in the object at `path`, or `None` when it is not there.
///
/// # Errors
/// [`JsoncError`] when the text is not an object or cannot be read.
pub fn member<'a>(text: &'a str, path: &[&str], name: &str) -> Result<Option<&'a str>, JsoncError> {
    let text = if text.trim().is_empty() { "{}" } else { text };
    let scan = Scan { bytes: text.as_bytes() };
    Ok(object_at(&scan, path)?.and_then(|object| object.members.into_iter().find(|member| member.key == name)).map(|member| &text[member.value]))
}

/// The white space at the start of the line that holds `at`.
fn indent_of(text: &str, at: usize) -> &str {
    let line = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
    let end = text[line..].find(|character: char| character != ' ' && character != '\t').map_or(text.len(), |end| line + end);
    &text[line..end.min(at)]
}

/// `value` with each line after the first moved in by `indent`.
fn indented(value: &str, indent: &str) -> String {
    value.replace('\n', &format!("\n{indent}"))
}

/// `"name": value` put into `object` as its last member.
fn insert(text: &str, object: &Object, name: &str, value: &str) -> String {
    let key = serde_json::Value::from(name).to_string();
    match object.members.last() {
        None => {
            let outer = indent_of(text, object.open);
            let inner = format!("{outer}  ");
            format!("{}\n{inner}{key}: {}\n{outer}{}", &text[..=object.open], indented(value, &inner), &text[object.close..])
        }
        Some(last) => {
            let inner = indent_of(text, last.span.start);
            let member = format!("\n{inner}{key}: {}", indented(value, inner));
            match last.comma {
                // The file ends its members with a comma, so the new member does also.
                Some(comma) => format!("{}{member},{}", &text[..=comma], &text[comma + 1..]),
                None => format!("{},{member}{}", &text[..last.span.end], &text[last.span.end..]),
            }
        }
    }
}

/// The text with `name` in the object at `path` set to `value` (JSON text, as `serde_json` prints
/// it with two spaces). An object of the path that is not there is made. Only that member changes.
///
/// # Errors
/// [`JsoncError`] when the text is not an object, cannot be read, or holds a different kind of
/// value at the path.
pub fn set(text: &str, path: &[&str], name: &str, value: &str) -> Result<String, JsoncError> {
    let text = if text.trim().is_empty() { "{}\n" } else { text };
    let scan = Scan { bytes: text.as_bytes() };
    // The deepest object of the path that is there; the rest of the path is made inside the value.
    for depth in (0..=path.len()).rev() {
        let Some(object) = object_at(&scan, &path[..depth])? else { continue };
        if depth == path.len() {
            if let Some(found) = object.members.iter().find(|member| member.key == name) {
                let inner = indent_of(text, found.span.start);
                return Ok(format!("{}{}{}", &text[..found.value.start], indented(value, inner), &text[found.value.end..]));
            }
            return Ok(insert(text, &object, name, value));
        }
        let mut nested = format!("{{\n  {}: {}\n}}", serde_json::Value::from(name), indented(value, "  "));
        for key in path[depth + 1..].iter().rev() {
            nested = format!("{{\n  {}: {}\n}}", serde_json::Value::from(*key), indented(&nested, "  "));
        }
        return Ok(insert(text, &object, path[depth], &nested));
    }
    Err(JsoncError::NotAnObject)
}

/// The text with `name` taken out of the object at `path`, or `None` when it is not there. The
/// objects of the path stay, also when they are then empty.
///
/// # Errors
/// [`JsoncError`] when the text is not an object or cannot be read.
pub fn remove(text: &str, path: &[&str], name: &str) -> Result<Option<String>, JsoncError> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let scan = Scan { bytes: text.as_bytes() };
    let Some(object) = object_at(&scan, path)? else { return Ok(None) };
    let Some(at) = object.members.iter().position(|member| member.key == name) else { return Ok(None) };
    let found = &object.members[at];
    let cut = match (found.comma, at.checked_sub(1).map(|before| &object.members[before])) {
        // A comma follows: the member goes with its comma and with the line break before it.
        (Some(comma), _) => line_start(text, found.span.start)..comma + 1,
        // The last member of several: it goes with the comma that the member before it then loses.
        (None, Some(before)) => before.span.end..found.span.end,
        // The only member: the object is then empty.
        (None, None) => object.open + 1..object.close,
    };
    Ok(Some(format!("{}{}", &text[..cut.start], &text[cut.end..])))
}

/// The index of the line break before `at` when only white space is between them, else `at`.
fn line_start(text: &str, at: usize) -> usize {
    let before = text[..at].trim_end_matches([' ', '\t']);
    match before.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').map_or(line.len(), str::len),
        None => at,
    }
}

#[cfg(test)]
mod tests;
