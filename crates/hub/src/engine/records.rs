//! How one record names another without an import: the record reference and the shared
//! subject keys (K-E4; LNK-001, LNK-004, TML-002).

use std::fmt;

use chrono::NaiveDate;

use crate::ids::{PersonId, PlaceId};

/// The longest record kind, in characters.
const MAX_KIND: usize = 40;
/// The longest record identifier, in characters.
const MAX_ID: usize = 200;

/// The text is not a record reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a record reference is a kind, a colon and an identifier, such as person:1234")]
pub struct BadReference;

/// The name of one record: `kind:id`. The kind says which sort of record it is; the
/// identifier is the owner's and nothing else reads meaning into it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Reference(String);

fn is_kind(kind: &str) -> bool {
    let mut chars = kind.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && kind.len() <= MAX_KIND
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().count() <= MAX_ID
        && id.trim() == id
        && !id.chars().any(char::is_control)
}

impl Reference {
    /// The reference with this exact text.
    ///
    /// # Errors
    /// [`BadReference`] when the kind is not lower snake case of at most 40 characters, or the
    /// identifier is empty, longer than 200 characters, starts or ends with a space, or holds
    /// a control character.
    pub fn parse(text: &str) -> Result<Self, BadReference> {
        let (kind, id) = text.split_once(':').ok_or(BadReference)?;
        if is_kind(kind) && is_id(id) {
            Ok(Self(text.to_owned()))
        } else {
            Err(BadReference)
        }
    }

    /// The reference to the record `id` of `kind`.
    ///
    /// # Errors
    /// [`BadReference`], as [`Reference::parse`].
    pub fn new(kind: &str, id: &str) -> Result<Self, BadReference> {
        Self::parse(&format!("{kind}:{id}"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The part before the first colon.
    #[must_use]
    pub fn kind(&self) -> &str {
        self.0.split_once(':').map_or("", |(kind, _)| kind)
    }

    /// The part after the first colon.
    #[must_use]
    pub fn id(&self) -> &str {
        self.0.split_once(':').map_or("", |(_, id)| id)
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A subject two extensions can both name, so their records meet at it (LNK-004).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Subject {
    /// A GitHub repository, by `owner/name` in lower case.
    Repository(String),
    Place(PlaceId),
    Person(PersonId),
    /// A calendar date with no time zone: it is not an instant.
    Date(NaiveDate),
}

fn is_repository_part(part: &str, longest: usize) -> bool {
    !part.is_empty()
        && part.len() <= longest
        && part != "."
        && part != ".."
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl Subject {
    pub const REPOSITORY: &'static str = "repository";
    pub const PLACE: &'static str = "place";
    pub const PERSON: &'static str = "person";
    pub const DATE: &'static str = "date";

    /// The repository `owner/name`, in any letter case; `None` for other text.
    #[must_use]
    pub fn repository(name: &str) -> Option<Self> {
        let (owner, repository) = name.split_once('/')?;
        (is_repository_part(owner, 39) && is_repository_part(repository, 100))
            .then(|| Self::Repository(name.to_ascii_lowercase()))
    }

    /// The one reference that names this subject.
    ///
    /// # Panics
    /// Never: each subject's text is a valid identifier.
    #[must_use]
    pub fn reference(&self) -> Reference {
        let (kind, id) = match self {
            Self::Repository(name) => (Self::REPOSITORY, name.clone()),
            Self::Place(id) => (Self::PLACE, id.to_string()),
            Self::Person(id) => (Self::PERSON, id.to_string()),
            Self::Date(date) => (Self::DATE, date.to_string()),
        };
        Reference::new(kind, &id).expect("a subject's text is a valid identifier")
    }

    /// The subject `reference` names. `None` when its kind is not a subject kind, or its
    /// identifier is not the exact text [`Subject::reference`] makes.
    #[must_use]
    pub fn of(reference: &Reference) -> Option<Self> {
        let id = reference.id();
        let subject = match reference.kind() {
            Self::REPOSITORY => Self::repository(id)?,
            Self::PLACE => Self::Place(id.parse().ok()?),
            Self::PERSON => Self::Person(id.parse().ok()?),
            Self::DATE => Self::Date(id.parse().ok()?),
            _ => return None,
        };
        (subject.reference() == *reference).then_some(subject)
    }

    /// True for a reference whose kind is a subject kind.
    #[must_use]
    pub fn owns(reference: &Reference) -> bool {
        [Self::REPOSITORY, Self::PLACE, Self::PERSON, Self::DATE].contains(&reference.kind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lnk_001_a_reference_is_a_kind_and_an_opaque_identifier() {
        let reference = Reference::parse("book:isbn:978-0-00").unwrap();
        assert_eq!(
            (reference.kind(), reference.id()),
            ("book", "isbn:978-0-00")
        );
        assert_eq!(reference.to_string(), "book:isbn:978-0-00");
        assert_eq!(Reference::new("event", "7"), Reference::parse("event:7"));
    }

    #[test]
    fn lnk_001_text_that_is_not_a_reference_is_refused() {
        let long_kind = format!("{}:1", "k".repeat(41));
        let long_id = format!("k:{}", "i".repeat(201));
        for bad in [
            "",
            "person",
            "person:",
            ":1",
            "Person:1",
            "1person:1",
            "per son:1",
            "person: 1",
            "person:1 ",
            "person:a\nb",
            &long_kind,
            &long_id,
        ] {
            assert_eq!(Reference::parse(bad), Err(BadReference), "{bad:?}");
        }
    }

    #[test]
    fn lnk_004_each_subject_has_one_reference_that_reads_back() {
        let person: PersonId = "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d".parse().unwrap();
        let place: PlaceId = "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8e".parse().unwrap();
        let subjects = [
            (
                Subject::repository("Okita/Anima.ing").unwrap(),
                "repository:okita/anima.ing",
            ),
            (
                Subject::Place(place),
                "place:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8e",
            ),
            (
                Subject::Person(person),
                "person:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d",
            ),
            (
                Subject::Date(NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()),
                "date:2026-11-16",
            ),
        ];
        for (subject, text) in subjects {
            let reference = subject.reference();
            assert_eq!(reference.as_str(), text);
            assert_eq!(Subject::of(&reference), Some(subject));
        }
    }

    #[test]
    fn lnk_004_a_second_spelling_of_a_subject_is_not_that_subject() {
        for other in [
            "repository:Okita/Anima",
            "repository:okita",
            "repository:okita/../x",
            "person:0192F5A0-7C3E-7A52-9D1B-3F4E5A6B7C8D",
            "place:not-a-uuid",
            "date:2026-11-16T00:00:00Z",
            "date:16/11/2026",
            "book:1",
        ] {
            let reference = Reference::parse(other).unwrap();
            assert_eq!(Subject::of(&reference), None, "{other}");
        }
        assert!(Subject::owns(&Reference::parse("date:tomorrow").unwrap()));
        assert!(!Subject::owns(&Reference::parse("book:1").unwrap()));
    }
}
