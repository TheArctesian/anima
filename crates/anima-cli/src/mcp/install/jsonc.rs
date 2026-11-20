//! One member of a JSON object, set or removed in the text of a file, with every other byte kept
//! (MCP-030). The files of other programs hold comments, trailing commas and an order that a
//! parser and a writer would lose, so this module only finds the place; `edit` cuts the text there.

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

pub(super) struct Member {
    pub(super) key: String,
    /// From the opening quote of the key to the end of the value.
    pub(super) span: Range<usize>,
    pub(super) value: Range<usize>,
    /// The index of the comma after the value, when there is one.
    pub(super) comma: Option<usize>,
}

pub(super) struct Object {
    /// The index of `{` and of `}`.
    pub(super) open: usize,
    pub(super) close: usize,
    pub(super) members: Vec<Member>,
}

pub(super) struct Scan<'a> {
    pub(super) bytes: &'a [u8],
}

impl Scan<'_> {
    /// The index of the first byte at or after `at` that is not space and not a comment.
    fn trivia(&self, mut at: usize) -> Result<usize, JsoncError> {
        loop {
            match (self.bytes.get(at), self.bytes.get(at + 1)) {
                (Some(b' ' | b'\t' | b'\n' | b'\r'), _) => at += 1,
                (Some(b'/'), Some(b'/')) => {
                    at = self.bytes[at..]
                        .iter()
                        .position(|byte| *byte == b'\n')
                        .map_or(self.bytes.len(), |end| at + end);
                }
                (Some(b'/'), Some(b'*')) => {
                    let end = self.bytes[at + 2..]
                        .windows(2)
                        .position(|pair| pair == b"*/")
                        .ok_or(JsoncError::Malformed)?;
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
                    .position(|byte| {
                        matches!(
                            byte,
                            b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r' | b'/'
                        )
                    })
                    .map_or(self.bytes.len(), |end| at + end);
                if end == at {
                    Err(JsoncError::Malformed)
                } else {
                    Ok(end)
                }
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
                Some(b'}') => {
                    return Ok(Object {
                        open,
                        close: at,
                        members,
                    });
                }
                Some(b'"') => {}
                _ => return Err(JsoncError::Malformed),
            }
            let key_end = self.string(at)?;
            let key = std::str::from_utf8(&self.bytes[at..key_end])
                .ok()
                .and_then(|text| serde_json::from_str(text).ok())
                .ok_or(JsoncError::Malformed)?;
            let colon = self.trivia(key_end)?;
            if self.bytes.get(colon) != Some(&b':') {
                return Err(JsoncError::Malformed);
            }
            let start = self.trivia(colon + 1)?;
            let end = self.value(start, depth + 1)?;
            let after = self.trivia(end)?;
            let comma = (self.bytes.get(after) == Some(&b',')).then_some(after);
            if comma.is_none() && self.bytes.get(after) != Some(&b'}') {
                return Err(JsoncError::Malformed);
            }
            members.push(Member {
                key,
                span: at..end,
                value: start..end,
                comma,
            });
            at = match comma {
                Some(comma) => self.trivia(comma + 1)?,
                None => after,
            };
        }
    }
}

/// The member `key` of `object`, or `None`. A key that is there twice is refused: this program
/// would change the first and the agent would read the last.
pub(super) fn only<'a>(object: &'a Object, key: &str) -> Result<Option<&'a Member>, JsoncError> {
    let mut found = object.members.iter().filter(|member| member.key == key);
    match (found.next(), found.next()) {
        (first, None) => Ok(first),
        _ => Err(JsoncError::Malformed),
    }
}

/// The object at `path` below the top level, or `None` when a key of the path is not there.
pub(super) fn object_at(scan: &Scan<'_>, path: &[&str]) -> Result<Option<Object>, JsoncError> {
    let start = scan.trivia(0)?;
    if scan.bytes.get(start) != Some(&b'{') {
        return Err(JsoncError::NotAnObject);
    }
    let mut object = scan.object(start, 0)?;
    if scan.trivia(object.close + 1)? != scan.bytes.len() {
        return Err(JsoncError::Malformed);
    }
    for key in path {
        let Some(member) = only(&object, key)? else {
            return Ok(None);
        };
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
    let scan = Scan {
        bytes: text.as_bytes(),
    };
    let Some(object) = object_at(&scan, path)? else {
        return Ok(None);
    };
    Ok(only(&object, name)?.map(|member| &text[member.value.clone()]))
}

mod edit;
pub use edit::{remove, set};

#[cfg(test)]
mod tests;
