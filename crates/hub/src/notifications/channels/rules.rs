//! Field rules more than one kind uses: https URLs and server origins (CR-14 items 6 and 7),
//! secret text, and the collector of field errors. No rule repeats the value it refused.

use reqwest::Url;

use crate::{api_errors::FieldError, credentials::Secret};

/// The longest URL accepted, in bytes.
const URL_MAX: usize = 2048;

/// The field errors of one write, gathered so the operator sees them all at once.
#[derive(Default)]
pub(super) struct Problems(Vec<FieldError>);

impl Problems {
    pub(super) fn add(&mut self, field: &'static str, message: impl Into<String>) {
        self.0.push(FieldError::new(field, message));
    }

    /// The value, or `None` after keeping its error.
    pub(super) fn take<T>(&mut self, result: Result<T, FieldError>) -> Option<T> {
        result.map_err(|error| self.0.push(error)).ok()
    }

    /// Keeps the error of a rule that gives no value.
    pub(super) fn note(&mut self, result: Result<(), FieldError>) {
        if let Err(error) = result {
            self.0.push(error);
        }
    }

    pub(super) fn into_errors(self) -> Vec<FieldError> {
        self.0
    }

    /// `Ok(())` when nothing was refused.
    ///
    /// # Errors
    /// Every error gathered.
    pub(super) fn check(self) -> Result<(), Vec<FieldError>> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(self.0)
        }
    }
}

/// Whether the text has a control character, a space of any kind or an invisible character:
/// a URL parser drops tabs and line breaks silently, so they are refused before it runs.
pub(super) fn has_blank_or_control(text: &str) -> bool {
    text.chars()
        .any(|c| c.is_control() || c.is_whitespace() || crate::text::is_invisible(c))
}

/// An https URL with a host and no user name or password (CR-14 items 6 and 7).
///
/// # Errors
/// The field error that says what to fix.
pub(super) fn https_url(field: &'static str, text: &str) -> Result<Url, FieldError> {
    if text.is_empty() {
        return Err(FieldError::new(field, "Enter a URL."));
    }
    if text.len() > URL_MAX {
        return Err(FieldError::new(
            field,
            format!("Use at most {URL_MAX} characters."),
        ));
    }
    if has_blank_or_control(text) {
        return Err(FieldError::new(field, "Remove spaces and line breaks."));
    }
    let url = Url::parse(text).map_err(|_| FieldError::new(field, "Enter a valid https URL."))?;
    if url.scheme() != "https" {
        return Err(FieldError::new(field, "Use an https URL."));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(FieldError::new(field, "Enter a URL with a host."));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(FieldError::new(
            field,
            "Remove the user name and password from the URL.",
        ));
    }
    Ok(url)
}

/// The origin of an https server given as an origin only: no path other than `/`, no query
/// and no fragment (CR-14 item 7). The origin is `https://host` with the port only when it is
/// not 443, so two ways of writing one server compare equal.
///
/// # Errors
/// The field error that says what to fix.
pub(super) fn server_origin(field: &'static str, text: &str) -> Result<String, FieldError> {
    let url = https_url(field, text)?;
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err(FieldError::new(
            field,
            "Enter only the server's address, such as https://ntfy.sh.",
        ));
    }
    Ok(origin(&url))
}

/// `https://host[:port]`, the port only when it is not the scheme's default.
pub(super) fn origin(url: &Url) -> String {
    url.origin().ascii_serialization()
}

/// The secret as text. Every secret arrives as a GraphQL string, so this fails only for bytes
/// that never came from the API.
///
/// # Errors
/// A field error that names the field.
pub(super) fn secret_text<'a>(
    field: &'static str,
    secret: &'a Secret,
) -> Result<&'a str, FieldError> {
    std::str::from_utf8(secret.expose()).map_err(|_| FieldError::new(field, "Enter text."))
}

/// Whether the text is empty: the update's way to clear an optional secret.
pub(super) fn is_empty(secret: &Secret) -> bool {
    secret.expose().is_empty()
}
