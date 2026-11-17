//! A link between two records and its parts (K-E4; LNK-001, LNK-002).

use chrono::{DateTime, Utc};

use crate::{engine::records::Reference, ids::RecordLinkId};

/// The longest link kind and the longest label, in characters.
const MAX_KIND: usize = 40;
const MAX_LABEL: usize = 200;

/// What a link means, in lower snake case, such as `lent_to`. The extension that makes the
/// link names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relation(String);

impl Relation {
    /// The relation with this text; `None` unless it is lower snake case of 1 to 40 characters
    /// that starts with a letter.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars();
        let valid = chars.next().is_some_and(|first| first.is_ascii_lowercase())
            && text.len() <= MAX_KIND
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        valid.then(|| Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The text cannot be a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a label is one line of at most 200 characters")]
pub struct BadLabel;

/// The text a link shows while its target has no record (LNK-002): trimmed, one line, at most
/// 200 characters.
///
/// # Errors
/// [`BadLabel`] for text with a control character or more than 200 characters. Blank text is
/// no label.
pub fn label(text: &str) -> Result<Option<String>, BadLabel> {
    let text = text.trim();
    if text.chars().any(char::is_control) || text.chars().count() > MAX_LABEL {
        Err(BadLabel)
    } else {
        Ok(Some(text.to_owned()).filter(|label| !label.is_empty()))
    }
}

/// A link to make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewLink {
    pub source: Reference,
    pub target: Reference,
    pub relation: Relation,
    pub label: Option<String>,
}

/// A stored link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub id: RecordLinkId,
    pub source: Reference,
    pub target: Reference,
    pub relation: Relation,
    /// Kept for as long as the link exists, also after the target has a record.
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// What [`connect`](super::connect) did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The link is new.
    Made,
    /// The link existed; it has a new label.
    Relabelled,
    /// The link existed as asked. Nothing was written.
    Kept,
}
