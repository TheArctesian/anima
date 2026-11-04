//! Device inventory: sites, devices, tags and SSH settings.

mod model;
mod page;
mod rules;
#[cfg(test)]
mod tests;

use crate::ids::{DeviceId, SiteId};

pub use model::{Device, OperatingSystem, Site, SshAuth, SshSettings};
pub use rules::FieldError;

/// What a lookup could not find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Device(DeviceId),
    Site(SiteId),
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(id) => write!(f, "device {id}"),
            Self::Site(id) => write!(f, "site {id}"),
        }
    }
}

/// Why a devices operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// The names of the fields in error, never their values.
fn field_names(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(|error| error.field)
        .collect::<Vec<_>>()
        .join(", ")
}
