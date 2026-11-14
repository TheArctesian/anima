//! Text an operator writes: titles, bodies, labels and workflow inputs. It only ever goes into
//! a JSON body, never into a path, and is bounded and free of NUL and control characters.

use std::collections::HashSet;

use super::PartError;

const TITLE_LONGEST: usize = 256;
const TEXT_LONGEST: usize = 65_536;
const DESCRIPTION_LONGEST: usize = 140;
const LABEL_LONGEST: usize = 50;
const LABELS_MOST: usize = 20;
const INPUT_NAME_LONGEST: usize = 100;
const INPUT_VALUE_LONGEST: usize = 1024;
/// The most inputs a dispatch may carry. With the name and value caps above, 25 inputs hold
/// at most 28,100 characters, under the 65,535 GitHub allows in all (decisions item 11).
const INPUTS_MOST: usize = 25;

/// Whether text may span lines: tab, line feed and carriage return are its only control
/// characters. Text on one line has none.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lines {
    One,
    Many,
}

/// Whether `text` has `least` to `most` characters and no control character but those `lines`
/// allows. NUL is a control character, so it is always refused.
fn is_bounded(text: &str, least: usize, most: usize, lines: Lines) -> bool {
    if text.len() > most.saturating_mul(4) {
        return false;
    }
    let allowed =
        |c: char| !c.is_control() || (lines == Lines::Many && matches!(c, '\t' | '\n' | '\r'));
    let count = text.chars().count();
    (least..=most).contains(&count) && text.chars().all(allowed)
}

macro_rules! bounded_text {
    ($(#[$doc:meta])* $name:ident, $least:expr, $most:expr, $lines:expr, $error:expr) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            /// # Errors
            /// [`PartError`] when the text is out of bounds or holds a refused character.
            pub fn parse(text: &str) -> Result<Self, PartError> {
                if is_bounded(text, $least, $most, $lines) {
                    Ok(Self(text.to_owned()))
                } else {
                    Err($error)
                }
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

bounded_text!(
    /// An issue title: 1 to 256 characters on one line.
    Title, 1, TITLE_LONGEST, Lines::One, PartError::Title
);
bounded_text!(
    /// An issue or comment body: 1 to 65,536 characters; lines allowed.
    Text, 1, TEXT_LONGEST, Lines::Many, PartError::Text
);
bounded_text!(
    /// A commit status description: at most 140 characters on one line.
    StatusDescription, 0, DESCRIPTION_LONGEST, Lines::One, PartError::StatusDescription
);
bounded_text!(
    /// One issue label: 1 to 50 characters on one line.
    Label, 1, LABEL_LONGEST, Lines::One, PartError::Label
);
bounded_text!(
    /// One workflow input value: at most 1,024 characters; lines allowed. Sent as a string.
    InputValue, 0, INPUT_VALUE_LONGEST, Lines::Many, PartError::InputValue
);

/// The labels of a new issue: at most 20.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Labels(Vec<Label>);

impl Labels {
    /// # Errors
    /// [`PartError::Label`] for more than 20 labels.
    pub fn new(labels: Vec<Label>) -> Result<Self, PartError> {
        if labels.len() <= LABELS_MOST {
            Ok(Self(labels))
        } else {
            Err(PartError::Label)
        }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Label] {
        &self.0
    }
}

/// The name of a workflow input: 1 to 100 of letters, digits, `-` and `_`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InputName(String);

impl InputName {
    /// # Errors
    /// [`PartError::InputName`] for an empty name, one over 100 bytes or any other character.
    pub fn parse(text: &str) -> Result<Self, PartError> {
        let plain = text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if !text.is_empty() && text.len() <= INPUT_NAME_LONGEST && plain {
            Ok(Self(text.to_owned()))
        } else {
            Err(PartError::InputName)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The inputs of a workflow dispatch: at most 25, each name once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inputs(Vec<(InputName, InputValue)>);

impl Inputs {
    /// # Errors
    /// [`PartError::Inputs`] for more than 25 inputs or a name given twice.
    pub fn new(inputs: Vec<(InputName, InputValue)>) -> Result<Self, PartError> {
        let mut names = HashSet::new();
        let unique = inputs.iter().all(|(name, _)| names.insert(name.as_str()));
        if inputs.len() <= INPUTS_MOST && unique {
            Ok(Self(inputs))
        } else {
            Err(PartError::Inputs)
        }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[(InputName, InputValue)] {
        &self.0
    }
}
