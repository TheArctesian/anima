//! The one redaction of secrets in text (RED-001, LOG-007): the hub's event and audit text, an
//! upload, and the text a source device sends all use it, so a new secret shape is added once.
//!
//! Pure: no file, no clock, no log. It prefers over-matching to letting a secret through.
//! Redact before the text is sealed or clipped (RED-003): a clipped secret no longer has its
//! shape. A new shape needs a new line in `fixtures/secret-shapes.txt` (RED-004).

mod document;
mod names;
mod provider_keys;
#[cfg(test)]
mod tests;
mod text;

pub use document::redact_document;
pub use names::{Secrecy, normalise, secrecy};
pub use text::redact_text;

/// The value that replaces a secret.
pub const REDACTED: &str = "[redacted]";
