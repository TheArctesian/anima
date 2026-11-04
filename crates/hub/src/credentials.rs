//! Device credentials: secrets sealed with the hub key before they reach storage (SEC-003).
//!
//! [`Vault`] holds the key from `ANIMA_KEY_FILE` (SEC-004) and seals each [`Secret`] with
//! XChaCha20-Poly1305 into a [`Sealed`] value. Plaintext never leaves this module except
//! through [`Secret::expose`]; secret types implement no format or serialise traits, and
//! this module writes no logs (LOG-007).
//!
//! SSH credentials live in `credentials.secrets`, one sealed row per part, each bound to its
//! device and kind (R-010). Only sessions read them back ([`load_ssh`]); the API sees
//! [`has_ssh`] alone (SEC-005).

mod secret;
mod ssh;
mod store;
mod vault;

#[cfg(test)]
mod tests;

use sqlx::{Connection as _, PgConnection, PgPool};

use crate::ids::DeviceId;

pub use secret::Secret;
pub use ssh::SshSecret;
pub use vault::{Sealed, Vault};

/// Why a credential operation failed. Messages name no path, key, nonce or plaintext.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The key file could not be read.
    #[error("the credential key file cannot be read ({0})")]
    KeyFile(std::io::ErrorKind),
    /// The key file does not hold a base64-encoded 32-byte key.
    #[error("the credential key file does not hold a base64-encoded 32-byte key")]
    KeyFormat,
    /// The sealed value names a key other than the vault's.
    #[error("the secret was sealed with a different key")]
    WrongKey,
    /// Authentication failed: the key, nonce, ciphertext or context does not match.
    #[error("the secret cannot be decrypted")]
    Decrypt,
    /// The sealed value is not in a known layout.
    #[error("the sealed secret is malformed")]
    Malformed,
    /// The secret store failed; the source holds the database error, never a secret.
    #[error("the credential store is unavailable")]
    Database(#[from] sqlx::Error),
    /// No device has the given id.
    #[error("the device does not exist")]
    UnknownDevice,
}

/// Replaces every SSH secret of the device with `secret`, sealed per part, in the caller's
/// connection. Runs in its own transaction (a savepoint inside the caller's), so a failure
/// leaves the previous secret in place.
///
/// Concurrent stores and clears for one device are serialised: each waits for this domain's
/// per-device advisory lock, held until the caller's transaction ends, so two writers never
/// leave a mixed row set.
///
/// Lock order for callers: call this before taking `FOR UPDATE` on, or deleting, the device
/// row in the same transaction. Otherwise a concurrent store can deadlock, which PostgreSQL
/// detects and reports as [`Error::Database`] (SQLSTATE 40P01).
///
/// # Errors
///
/// [`Error::UnknownDevice`] when no device has this id, [`Error::Database`] otherwise.
pub async fn store_ssh(
    conn: &mut PgConnection,
    vault: &Vault,
    device: DeviceId,
    secret: SshSecret,
) -> Result<(), Error> {
    let sealed: Vec<_> = secret
        .into_parts()
        .into_iter()
        .map(|(kind, part)| (kind, vault.seal(&part, &ssh::context(device, kind))))
        .collect();
    let mut tx = conn.begin().await?;
    store::lock_device(&mut tx, device).await?;
    store::delete_device(&mut tx, device).await?;
    for (kind, value) in &sealed {
        store::insert(&mut tx, device, *kind, value).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Opens the device's SSH secret; `None` when none is stored.
///
/// # Errors
///
/// [`Error::WrongKey`] or [`Error::Decrypt`] when a row does not open with this vault for this
/// device, [`Error::Malformed`] for a stored row set no [`SshSecret`] produces, and
/// [`Error::Database`].
pub async fn load_ssh(
    db: &PgPool,
    vault: &Vault,
    device: DeviceId,
) -> Result<Option<SshSecret>, Error> {
    let rows = store::fetch_device(db, device).await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let parts = rows
        .into_iter()
        .map(|row| {
            let kind = ssh::SshKind::from_sql(&row.kind)?;
            let sealed = Sealed::from_parts(&row.nonce, row.ciphertext, row.key_id)?;
            Ok((kind, vault.open(&sealed, &ssh::context(device, kind))?))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    SshSecret::from_parts(parts).map(Some)
}

/// Whether the device has an SSH secret stored.
///
/// # Errors
///
/// [`Error::Database`] when the store cannot be read.
pub async fn has_ssh(db: &PgPool, device: DeviceId) -> Result<bool, Error> {
    store::exists_for_device(db, device).await
}

/// Deletes every SSH secret of the device in the caller's connection. Returns true if a
/// secret was removed (R-010). Waits for a concurrent [`store_ssh`] for the device, as that does.
///
/// Lock order for callers: call this before taking `FOR UPDATE` on, or deleting, the device
/// row in the same transaction. Otherwise a concurrent store can deadlock, which PostgreSQL
/// detects and reports as [`Error::Database`] (SQLSTATE 40P01).
///
/// # Errors
///
/// [`Error::Database`] when the store cannot be written.
pub async fn clear_ssh(conn: &mut PgConnection, device: DeviceId) -> Result<bool, Error> {
    let mut tx = conn.begin().await?;
    store::lock_device(&mut tx, device).await?;
    let removed = store::delete_device(&mut tx, device).await?;
    tx.commit().await?;
    Ok(removed > 0)
}
