//! The names of the library: model names and variant tags in the Ollama form `name:tag`
//! (LIBM-006), aliases (LIBM-007), and Hugging Face repositories with their organisation
//! (LIBM-003). Pure rules; the messages hold no input value (LOG-007).

use crate::api_errors::FieldError;

/// The longest model name, tag or alias.
const LONGEST: usize = 64;
/// The longest organisation or repository name part on Hugging Face.
const LONGEST_PART: usize = 96;

/// A model name, a tag or an alias: 1 to 64 of `[a-z0-9._-]`, starting with a letter or digit.
#[must_use]
pub fn is_name(text: &str) -> bool {
    let bytes = text.as_bytes();
    (1..=LONGEST).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|each| each.is_ascii_lowercase() || each.is_ascii_digit() || b"._-".contains(each))
}

/// The checked model name.
///
/// # Errors
/// A [`FieldError`] on `field` when the text breaks the name rule.
pub fn name(field: &'static str, text: &str) -> Result<String, FieldError> {
    let text = text.trim();
    if is_name(text) {
        Ok(text.to_owned())
    } else {
        Err(FieldError::new(
            field,
            "Use 1 to 64 lower-case letters, digits, dots, dashes or underscores.",
        ))
    }
}

/// `name:tag`, the address of one variant (LIBM-006).
#[must_use]
pub fn address(name: &str, tag: &str) -> String {
    format!("{name}:{tag}")
}

/// The model name of a request: `name` or `name:tag` give `name`.
#[must_use]
pub fn model_part(text: &str) -> &str {
    text.split_once(':').map_or(text, |(name, _)| name)
}

/// A Hugging Face repository `organisation/name`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    pub organisation: String,
    pub name: String,
}

impl Repository {
    #[must_use]
    pub fn full(&self) -> String {
        format!("{}/{}", self.organisation, self.name)
    }
}

/// One part of a repository name: 1 to 96 of `[A-Za-z0-9._-]`, not starting with a dot.
fn is_part(text: &str) -> bool {
    (1..=LONGEST_PART).contains(&text.len())
        && !text.starts_with('.')
        && text
            .bytes()
            .all(|each| each.is_ascii_alphanumeric() || b"._-".contains(&each))
}

/// The checked organisation name, as Hugging Face writes it.
///
/// # Errors
/// A [`FieldError`] on `field` when the text is not an organisation name.
pub fn organisation(field: &'static str, text: &str) -> Result<String, FieldError> {
    let text = text.trim();
    if is_part(text) {
        Ok(text.to_owned())
    } else {
        Err(FieldError::new(
            field,
            "Use the name of a Hugging Face organisation, such as unsloth.",
        ))
    }
}

/// The checked repository.
///
/// # Errors
/// A [`FieldError`] on `repository` when the text is not `organisation/name`.
pub fn repository(text: &str) -> Result<Repository, FieldError> {
    let refused = || {
        FieldError::new(
            "repository",
            "Use a Hugging Face repository as organisation/name.",
        )
    };
    let (organisation, name) = text.trim().split_once('/').ok_or_else(refused)?;
    if is_part(organisation) && is_part(name) {
        Ok(Repository {
            organisation: organisation.to_owned(),
            name: name.to_owned(),
        })
    } else {
        Err(refused())
    }
}

/// The words a repository name adds after the model name, which a model name leaves out.
const SUFFIXES: [&str; 6] = ["-gguf", "-mlx", "-instruct-gguf", "-bf16", "-4bit", "-8bit"];

/// The model name the hub proposes for a repository: its name in lower case, without a format
/// word at the end, with each other character mapped to a dash.
#[must_use]
pub fn proposed(repository: &Repository) -> String {
    let mut text: String = repository
        .name
        .to_ascii_lowercase()
        .chars()
        .map(|each| {
            if each.is_ascii_alphanumeric() || each == '.' || each == '-' {
                each
            } else {
                '-'
            }
        })
        .collect();
    while let Some(suffix) = SUFFIXES.iter().find(|suffix| text.ends_with(*suffix)) {
        text.truncate(text.len() - suffix.len());
    }
    text.truncate(LONGEST);
    text.trim_end_matches(['-', '.']).to_owned()
}

/// Whether a variant from the repository's organisation can be added: the organisation is in
/// the approved list, or it is the model publisher (LIBM-003). Hugging Face compares
/// organisation names without case.
#[must_use]
pub fn is_approved(organisation: &str, approved: &[String], publisher: Option<&str>) -> bool {
    approved
        .iter()
        .map(String::as_str)
        .chain(publisher)
        .any(|each| each.eq_ignore_ascii_case(organisation))
}
