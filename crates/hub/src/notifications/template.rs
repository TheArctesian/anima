//! Message templates (NOT-007, D-21): placeholder substitution and nothing else.
//!
//! The placeholders are `{device}`, `{check}`, `{status}`, `{previous}`, `{message}`, `{at}`
//! and `{days}`. There is no condition, loop, filter or default. A template is parsed when it
//! is saved, so a mistake is a `VALIDATION` error then and never a failed send later.
//!
//! - A literal brace is written twice: `{{` is `{` and `}}` is `}`. `{{device}}` therefore
//!   shows the text `{device}` and expands nothing.
//! - An unknown name (also `{}` and `{ device }`), a `{` with no closing `}` and a `}` with
//!   no opening `{` are refused.
//! - A value that is missing renders as an empty string: `{days}` on a Down notice and
//!   `{message}` on a check with no message leave nothing behind.
//! - Rendering is one pass over the parsed template. A value is never parsed again, so a value
//!   that holds `{message}` shows those characters.
//!
//! Pure: no I/O, no clock.

#[cfg(test)]
mod tests;
mod values;

pub(super) use values::Values;

use crate::api_errors::FieldError;

/// The longest template title and body, in characters (K2 `title_template` and
/// `body_template` checks).
pub(super) const TITLE_MAX: usize = 500;
pub(super) const BODY_MAX: usize = 4000;

/// The longest unknown name an error message repeats.
const NAME_SHOWN: usize = 30;

/// A value a template can show (D-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Placeholder {
    Device,
    Check,
    Status,
    Previous,
    Message,
    At,
    Days,
}

impl Placeholder {
    const ALL: [Self; 7] = [
        Self::Device,
        Self::Check,
        Self::Status,
        Self::Previous,
        Self::Message,
        Self::At,
        Self::Days,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Device => "device",
            Self::Check => "check",
            Self::Status => "status",
            Self::Previous => "previous",
            Self::Message => "message",
            Self::At => "at",
            Self::Days => "days",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// The placeholders as an operator types them, braces included.
pub(super) fn placeholder_texts() -> Vec<String> {
    Placeholder::ALL
        .into_iter()
        .map(|p| format!("{{{}}}", p.name()))
        .collect()
}

/// Why a template is refused. The messages quote at most a short name, never the template.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum Invalid {
    #[error("{{{0}}} is not a placeholder")]
    Unknown(String),
    #[error("a {{ has no closing }}")]
    Unclosed,
    #[error("a }} has no opening {{")]
    Stray,
}

enum Piece {
    Text(String),
    Field(Placeholder),
}

/// A template that passed the grammar above.
pub(super) struct Template(Vec<Piece>);

impl Template {
    /// # Errors
    /// `Unknown`, `Unclosed` or `Stray`, for the first mistake in reading order.
    pub(super) fn parse(text: &str) -> Result<Self, Invalid> {
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.next_if_eq(&'{').is_some() => literal.push('{'),
                '}' if chars.next_if_eq(&'}').is_some() => literal.push('}'),
                '}' => return Err(Invalid::Stray),
                '{' => {
                    let mut name = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some('{') | None => return Err(Invalid::Unclosed),
                            Some(c) => name.push(c),
                        }
                    }
                    let field = Placeholder::from_name(&name)
                        .ok_or_else(|| Invalid::Unknown(name.chars().take(NAME_SHOWN).collect()))?;
                    if !literal.is_empty() {
                        pieces.push(Piece::Text(std::mem::take(&mut literal)));
                    }
                    pieces.push(Piece::Field(field));
                }
                _ => literal.push(c),
            }
        }
        if !literal.is_empty() {
            pieces.push(Piece::Text(literal));
        }
        Ok(Self(pieces))
    }

    /// The template with each placeholder replaced by its value.
    pub(super) fn render(&self, values: &Values) -> String {
        self.0
            .iter()
            .map(|piece| match piece {
                Piece::Text(text) => text.clone(),
                Piece::Field(field) => values.get(*field),
            })
            .collect()
    }
}

/// A rendered title and body, at most `TITLE_MAX` and `BODY_MAX` characters. They may still
/// hold what the template's own text holds (a line break in the body, say): each channel
/// encodes them for itself (`encode`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Rendered {
    pub(super) title: String,
    pub(super) body: String,
}

/// Renders a title and body template with the values of one notification. A longer result is
/// cut at the limits above.
///
/// # Errors
/// `Invalid` when a stored template does not parse (it was validated when saved, so only a row
/// written outside the API can fail).
pub(super) fn render(title: &str, body: &str, values: &Values) -> Result<Rendered, Invalid> {
    let cut = |text: String, max: usize| text.chars().take(max).collect::<String>();
    Ok(Rendered {
        title: cut(Template::parse(title)?.render(values), TITLE_MAX),
        body: cut(Template::parse(body)?.render(values), BODY_MAX),
    })
}

/// The field errors of a template, named `template.title` and `template.body` as in K1's
/// `NotificationTemplateInput`: length first, then placeholders.
pub(super) fn field_errors(title: &str, body: &str) -> Vec<FieldError> {
    let mut errors = Vec::new();
    for (field, text, max) in [
        ("template.title", title, TITLE_MAX),
        ("template.body", body, BODY_MAX),
    ] {
        if text.chars().count() > max {
            errors.push(FieldError::new(
                field,
                format!("Use at most {max} characters."),
            ));
        } else if let Err(invalid) = Template::parse(text) {
            errors.push(FieldError::new(field, explain(&invalid)));
        }
    }
    errors
}

fn explain(invalid: &Invalid) -> String {
    let names = placeholder_texts().join(", ");
    format!("{invalid}. Use {names}. Write {{{{ for a {{ and }}}} for a }}.")
}
