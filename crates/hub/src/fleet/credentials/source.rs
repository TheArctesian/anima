//! The sealed form of a source account's credential (SRC-008, PRJ-018, SEC-003). Pure.
//!
//! The value is sealed under the context `credentials.source_secrets:<account_id>`, the account
//! id as hyphenated lowercase UUID text, so a row moved or copied to another account does not
//! open for it. The plaintext is the caller's bytes unchanged: what they mean (an App key, a
//! token) is the caller's concern.

use crate::ids::SourceAccountId;

use super::{Error, Sealed, Secret, Vault, store::StoredSourceSecret};

/// The context prefix of sealed source secrets (K3).
const CONTEXT: &str = "credentials.source_secrets";

/// The associated data that binds a sealed source secret to its account.
pub(super) fn context(account: SourceAccountId) -> Vec<u8> {
    format!("{CONTEXT}:{account}").into_bytes()
}

/// Seals `secret` for `account`.
///
/// # Errors
/// [`Error::Random`] when the OS random source fails.
pub(super) fn seal(
    vault: &Vault,
    account: SourceAccountId,
    secret: &Secret,
) -> Result<Sealed, Error> {
    vault.seal(secret, &context(account))
}

/// Opens a stored row for `account`.
///
/// # Errors
/// [`Error::Malformed`] for columns not in the sealed layout, [`Error::WrongKey`] when another
/// key sealed it, [`Error::Decrypt`] when it was sealed for another account or was changed.
pub(super) fn open(
    vault: &Vault,
    row: StoredSourceSecret,
    account: SourceAccountId,
) -> Result<Secret, Error> {
    let sealed = Sealed::from_parts(&row.nonce, row.ciphertext, row.key_id)?;
    vault.open(&sealed, &context(account))
}
