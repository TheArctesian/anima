//! What the rules of one kind give back: the settings and secrets to store, the secret fields
//! that changed, and the stored secrets an update may keep.

use super::{Secrets, Settings};
use crate::api_errors::FieldError;

/// What an update asks for when it keeps a secret that cannot be read (as checks does).
const UNOPENED: &str = "The stored secrets cannot be opened. Enter every secret again.";

/// The error of an update that keeps a stored secret it cannot read.
pub(super) fn unopened() -> Vec<FieldError> {
    vec![FieldError::new("settings", UNOPENED)]
}

/// The stored secrets of a channel, as the update found them.
pub(in crate::fleet::notifications) enum Kept<T> {
    /// Nothing is stored.
    Nothing,
    Opened(T),
    /// Something is stored and does not open with this vault for this channel and target.
    Unopened,
}

impl<T> Kept<T> {
    /// The same state with the opened value converted; a value of the wrong shape counts as
    /// unopened.
    pub(super) fn map<U>(self, convert: impl FnOnce(T) -> Option<U>) -> Kept<U> {
        match self {
            Self::Nothing => Kept::Nothing,
            Self::Opened(value) => convert(value).map_or(Kept::Unopened, Kept::Opened),
            Self::Unopened => Kept::Unopened,
        }
    }

    /// The stored secrets, which the change keeps. The settings said they exist, so a channel
    /// with nothing stored, or with secrets that do not open, asks for every secret again.
    ///
    /// # Errors
    /// A field error on `settings` that asks for every secret again.
    pub(super) fn opened(self) -> Result<T, Vec<FieldError>> {
        match self {
            Self::Opened(value) => Ok(value),
            Self::Nothing | Self::Unopened => Err(unopened()),
        }
    }
}

/// The secret fields a write set or cleared, by input field name, never by value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::fleet::notifications) struct Touched {
    pub(in crate::fleet::notifications) set: Vec<&'static str>,
    pub(in crate::fleet::notifications) cleared: Vec<&'static str>,
}

impl Touched {
    pub(super) fn set(&mut self, field: &'static str) {
        self.set.push(field);
    }

    pub(super) fn cleared(&mut self, field: &'static str) {
        self.cleared.push(field);
    }
}

/// The result of one kind's rules.
pub(super) struct Outcome {
    pub(super) settings: Settings,
    pub(super) secrets: Option<Secrets>,
    pub(super) touched: Touched,
}
