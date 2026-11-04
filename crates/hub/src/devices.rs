//! Device inventory: sites, devices, tags and SSH settings.

mod model;
mod page;
mod rules;
#[cfg(test)]
mod tests;

use crate::ids::{DeviceId, SiteId};

pub use model::{
    Device, DeviceChange, DeviceFilter, NewDevice, NewSite, OperatingSystem, Site, SshAuth,
    SshSettings, UnknownValue,
};
pub use page::{Page, decode_cursor, encode_cursor, page_after, page_size, split_page};
pub use rules::{
    DeviceChangeInput, FieldError, NewDeviceInput, NewSiteInput, SshInput, validate_change,
    validate_new_device, validate_new_site,
};

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
    #[error("input is not valid ({} field errors)", .0.len())]
    Validation(Vec<FieldError>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}
