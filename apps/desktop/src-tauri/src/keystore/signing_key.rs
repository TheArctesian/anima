//! The updater key, one per computer, in the OS key store (BLD-045): the key sealed with its
//! password (D-19), and beside it the public key line, so that showing and checking a signature
//! never reads the sealed key. Nothing here opens, reads aloud or logs either text.

use keyring::Entry;
use zeroize::Zeroizing;

use super::{error::KeystoreError, token::SERVICE};

const SEALED_ACCOUNT: &str = "updater-key";
const PUBLIC_ACCOUNT: &str = "updater-key-public";

fn read(account: &str) -> Result<Option<Zeroizing<String>>, KeystoreError> {
    match Entry::new(SERVICE, account)?.get_password() {
        Ok(text) => Ok(Some(Zeroizing::new(text))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write(account: &str, text: &str) -> Result<(), KeystoreError> {
    if text.is_empty() {
        return Err(KeystoreError::EmptyToken);
    }
    Entry::new(SERVICE, account)?
        .set_password(text)
        .map_err(KeystoreError::from)
}

/// The sealed key text, or `None` when no key is stored.
pub fn read_signing_key() -> Result<Option<Zeroizing<String>>, KeystoreError> {
    read(SEALED_ACCOUNT)
}

/// The public key line of the stored key, or `None` when no key is stored.
pub fn read_signing_public_key() -> Result<Option<String>, KeystoreError> {
    Ok(read(PUBLIC_ACCOUNT)?.map(|text| text.as_str().to_owned()))
}

/// Stores the sealed key and its public key line, replacing any earlier key. An empty text is
/// refused. The sealed key goes first, so a public line is never left without its key.
pub fn write_signing_key(sealed: &str, public_line: &str) -> Result<(), KeystoreError> {
    if public_line.is_empty() {
        return Err(KeystoreError::EmptyToken);
    }
    write(SEALED_ACCOUNT, sealed)?;
    write(PUBLIC_ACCOUNT, public_line)
}
