//! The updater key entries the T2 builder import wrote (WP-36 D-19), which nothing reads since
//! the hub signs (R-117, D-41). The shell deletes them so the sealed key does not outlive its use.

use keyring::Entry;

use super::{error::KeystoreError, token::SERVICE};

/// The accounts of the sealed updater key and of its public line, under [`SERVICE`].
pub(super) const RETIRED_ACCOUNTS: [&str; 2] = ["updater-key", "updater-key-public"];

/// Deletes both retired entries; one that is missing is already gone. Tries both before it
/// returns the first failure, which carries no backend text (LOG-007).
pub(super) fn forget_updater_key() -> Result<(), KeystoreError> {
    let mut first = Ok(());
    for account in RETIRED_ACCOUNTS {
        let deleted = Entry::new(SERVICE, account).and_then(|entry| entry.delete_credential());
        match deleted {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(error) => first = first.and(Err(error.into())),
        }
    }
    first
}
