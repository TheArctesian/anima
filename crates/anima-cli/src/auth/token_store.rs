//! Where `anima` keeps the token of each hub (CMD-012): the OS key store (service
//! `ing.anima.cli`, account the hub origin), or the `0600` token file. On Linux the file takes
//! over when there is no Secret Service; `ANIMA_TOKEN_STORE=file` chooses the file anywhere.
//!
//! No error here holds a token or the key store's own message, which could quote one (CMD-014).

use std::path::PathBuf;

use keyring::Entry;
use zeroize::Zeroizing;

use super::token_file::TokenFile;
use crate::hub::{Origin, Token};
use crate::private_file::PrivateFileError;

const SERVICE: &str = "ing.anima.cli";

#[derive(Debug)]
pub enum TokenStore {
    /// The OS key store, and the file to use when it is not there (Linux only).
    KeyStore {
        fallback: Option<TokenFile>,
    },
    File(TokenFile),
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("The key store {0}.")]
    KeyStore(&'static str),
    #[error("The token file {path}: {source}.")]
    File {
        path: PathBuf,
        source: PrivateFileError,
    },
    #[error("The token file {path} does not read; delete it and sign in again.")]
    Corrupt { path: PathBuf },
    #[error("The stored token is not valid; run `anima auth logout`, then sign in again.")]
    BadToken,
}

impl TokenStore {
    /// The file store when `file_only`, else this system's key store with the file behind it
    /// on Linux.
    #[must_use]
    pub fn for_system(file: Option<TokenFile>, file_only: bool) -> Self {
        match file {
            Some(file) if file_only => Self::File(file),
            file => Self::KeyStore {
                fallback: file.filter(|_| cfg!(target_os = "linux")),
            },
        }
    }

    /// The token stored for `origin`.
    ///
    /// # Errors
    /// [`StoreError`] when the store cannot be read.
    pub fn read(&self, origin: &Origin) -> Result<Option<Token>, StoreError> {
        match self {
            Self::File(file) => file.read(origin),
            Self::KeyStore { fallback } => match entry(origin)?.get_password() {
                Ok(text) => Token::new(Zeroizing::new(text))
                    .map(Some)
                    .ok_or(StoreError::BadToken),
                // A token written while the key store was away is in the file.
                Err(keyring::Error::NoEntry) => {
                    fallback.as_ref().map_or(Ok(None), |file| file.read(origin))
                }
                Err(error) => fall_back(&error, fallback.as_ref())?.read(origin),
            },
        }
    }

    /// Stores `token` for `origin`, replacing an earlier one.
    ///
    /// # Errors
    /// [`StoreError`] when the store cannot be written.
    pub fn write(&self, origin: &Origin, token: &Token) -> Result<(), StoreError> {
        match self {
            Self::File(file) => file.write(origin, token),
            Self::KeyStore { fallback } => match entry(origin)?.set_password(token.expose()) {
                Ok(()) => Ok(()),
                Err(error) => fall_back(&error, fallback.as_ref())?.write(origin, token),
            },
        }
    }

    /// Deletes the token for `origin`; whether there was one.
    ///
    /// # Errors
    /// [`StoreError`] when the store cannot be changed.
    pub fn delete(&self, origin: &Origin) -> Result<bool, StoreError> {
        match self {
            Self::File(file) => file.delete(origin),
            Self::KeyStore { fallback } => match entry(origin)?.delete_credential() {
                Ok(()) => Ok(true),
                Err(keyring::Error::NoEntry) => fallback
                    .as_ref()
                    .map_or(Ok(false), |file| file.delete(origin)),
                Err(error) => fall_back(&error, fallback.as_ref())?.delete(origin),
            },
        }
    }
}

fn entry(origin: &Origin) -> Result<Entry, StoreError> {
    Entry::new(SERVICE, &origin.as_text()).map_err(|error| StoreError::KeyStore(words(&error)))
}

/// The file, when the key store is missing or failed and there is one to fall back on.
fn fall_back<'file>(
    error: &keyring::Error,
    fallback: Option<&'file TokenFile>,
) -> Result<&'file TokenFile, StoreError> {
    let missing = matches!(
        *error,
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_)
    );
    match fallback {
        Some(file) if missing => Ok(file),
        _ => Err(StoreError::KeyStore(words(error))),
    }
}

/// The key store's failure in fixed words; its own message is never kept.
const fn words(error: &keyring::Error) -> &'static str {
    match error {
        keyring::Error::NoStorageAccess(_) => "is locked or not available",
        keyring::Error::PlatformFailure(_) => "failed",
        keyring::Error::BadEncoding(_) => "holds a token that is not text",
        _ => "refused the request",
    }
}
