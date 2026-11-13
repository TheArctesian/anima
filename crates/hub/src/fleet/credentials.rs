//! Device credentials: secrets sealed with the hub key before they reach storage (SEC-003).
//!
//! [`Vault`] holds the key from `ANIMA_KEY_FILE` (SEC-004) and seals each [`Secret`] with
//! XChaCha20-Poly1305 into a [`Sealed`] value. Plaintext never leaves this module except
//! through [`Secret::expose`]; secret types implement no format or serialise traits, and
//! this module writes no logs (LOG-007).
//!
//! SSH credentials and RDP and VNC passwords live in `credentials.secrets`, one sealed row per
//! SSH part or password, each bound to its device and kind, so a value moved to another row
//! does not open. Each function sees only its own kinds: an RDP or VNC password never counts
//! as, opens as, or is deleted with an SSH secret, nor the reverse. Only sessions read secrets
//! back ([`load_ssh`], [`load_password`]); the API sees only whether one exists ([`has_ssh`],
//! [`with_ssh`], [`with_password`]) (SEC-005).
//!
//! Each secret is bound to the device host it was entered for, sealed inside the value with it
//! (SEC-011). The readers take the host they are about to connect to and open a secret only for
//! that host, letter case aside; for any other host they return
//! [`Error::EnteredForOtherHost`] and the caller sends nothing. The host is the device's host
//! text, never a resolved address. A refused secret stays stored: it opens again if the device
//! goes back to its host, and storing it again binds it to the new one.
//!
//! Writers of every kind take one advisory lock per device, so the lock order documented on
//! each writer holds for all of them.

mod host;
mod password;
mod secret;
mod ssh;
mod store;
mod vault;

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use sqlx::{Connection as _, PgConnection, PgPool};

use crate::ids::DeviceId;

use self::ssh::SshKind;

pub use password::PasswordKind;
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
    /// The OS random source yielded no nonce, so nothing was sealed.
    #[error("the OS random source failed")]
    Random(#[source] getrandom::Error),
    /// The secret was entered for another host than the one the caller connects to; the
    /// opened secret was dropped (SEC-011). Names neither host (LOG-007).
    #[error("the credential was entered for another host of the device")]
    EnteredForOtherHost,
}

/// Replaces every SSH secret of the device with `secret`, sealed per part and bound to `host`,
/// the device's host it was entered for (SEC-011), in the caller's connection. Runs in its own
/// transaction (a savepoint inside the caller's), so a failure leaves the previous secret in
/// place.
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
/// [`Error::Random`] when the OS random source fails, [`Error::UnknownDevice`] when no device
/// has this id, [`Error::Database`] otherwise.
pub async fn store_ssh(
    conn: &mut PgConnection,
    vault: &Vault,
    (device, host): (DeviceId, &str),
    secret: SshSecret,
) -> Result<(), Error> {
    let sealed = secret
        .into_parts()
        .into_iter()
        .map(|(kind, part)| {
            Ok((
                kind,
                host::seal(vault, (device, host), kind.as_sql(), &part)?,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mut tx = conn.begin().await?;
    store::lock_device(&mut tx, device).await?;
    store::delete_kinds(&mut tx, device, &SshKind::SQL_KINDS).await?;
    for (kind, value) in &sealed {
        store::insert(&mut tx, device, kind.as_sql(), value).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Opens the device's SSH secret for a connection to `host`, the device's host the caller is
/// about to connect to; `None` when none is stored.
///
/// # Errors
///
/// [`Error::EnteredForOtherHost`] when it was entered for another host (SEC-011);
/// [`Error::WrongKey`] or [`Error::Decrypt`] when a row does not open with this vault for this
/// device, [`Error::Malformed`] for a stored row set no [`SshSecret`] produces, and
/// [`Error::Database`].
pub async fn load_ssh(
    db: &PgPool,
    vault: &Vault,
    (device, host): (DeviceId, &str),
) -> Result<Option<SshSecret>, Error> {
    let rows = store::fetch_kinds(db, device, &SshKind::SQL_KINDS).await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let parts = rows
        .into_iter()
        .map(|row| {
            let kind = SshKind::from_sql(&row.kind)?;
            let legacy = ssh::context(device, kind);
            Ok((kind, host::open(vault, row, (device, host), &legacy)?))
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
    store::exists_of_kinds(db, device, &SshKind::SQL_KINDS).await
}

/// Which of the given devices have an SSH secret stored, in one query, so a list of devices
/// costs one round trip (API-005). Devices without a secret, and unknown ids, are left out.
///
/// # Errors
///
/// [`Error::Database`] when the store cannot be read.
pub async fn with_ssh(db: &PgPool, devices: &[DeviceId]) -> Result<HashSet<DeviceId>, Error> {
    store::devices_with_kinds(db, devices, &SshKind::SQL_KINDS).await
}

/// Deletes every SSH secret of the device in the caller's connection. Returns true if a
/// secret was removed. Waits for a concurrent [`store_ssh`] for the device, as that does.
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
    let removed = store::delete_kinds(&mut tx, device, &SshKind::SQL_KINDS).await?;
    tx.commit().await?;
    Ok(removed > 0)
}

/// Replaces the device's password of `kind` with `secret`, sealed and bound to `host`, the
/// device's host it was entered for (SEC-011), in the caller's connection.
/// Runs in its own transaction (a savepoint inside the caller's), so a failure leaves the
/// previous password in place. Other kinds, SSH included, are left as they are.
///
/// Serialised with every other store and clear for the device, of any kind, by the same
/// per-device lock as [`store_ssh`], and with the same lock order for callers: call this before
/// taking `FOR UPDATE` on, or deleting, the device row in the same transaction.
///
/// # Errors
///
/// [`Error::Random`] when the OS random source fails, [`Error::UnknownDevice`] when no device
/// has this id, [`Error::Database`] otherwise.
pub async fn store_password(
    conn: &mut PgConnection,
    vault: &Vault,
    (device, host): (DeviceId, &str),
    (kind, secret): (PasswordKind, Secret),
) -> Result<(), Error> {
    let sealed = host::seal(vault, (device, host), kind.as_sql(), &secret)?;
    drop(secret);
    let mut tx = conn.begin().await?;
    store::lock_device(&mut tx, device).await?;
    store::delete_kinds(&mut tx, device, &[kind.as_sql()]).await?;
    store::insert(&mut tx, device, kind.as_sql(), &sealed).await?;
    tx.commit().await?;
    Ok(())
}

/// Opens the device's password of `kind` for a connection to `host`, the device's host the
/// caller is about to connect to; `None` when none is stored.
///
/// # Errors
///
/// [`Error::EnteredForOtherHost`] when it was entered for another host (SEC-011);
/// [`Error::WrongKey`] or [`Error::Decrypt`] when the row does not open with this vault for
/// this device and kind, [`Error::Malformed`] for a row not in the sealed layout, and
/// [`Error::Database`].
pub async fn load_password(
    db: &PgPool,
    vault: &Vault,
    (device, host): (DeviceId, &str),
    kind: PasswordKind,
) -> Result<Option<Secret>, Error> {
    // The unique (device_id, kind) constraint allows at most one row.
    let Some(row) = store::fetch_kinds(db, device, &[kind.as_sql()])
        .await?
        .pop()
    else {
        return Ok(None);
    };
    let legacy = password::context(device, kind);
    host::open(vault, row, (device, host), &legacy).map(Some)
}

/// Which of the given devices have a password of `kind` stored, in one query (API-005).
/// Devices without one, and unknown ids, are left out.
///
/// # Errors
///
/// [`Error::Database`] when the store cannot be read.
pub async fn with_password(
    db: &PgPool,
    devices: &[DeviceId],
    kind: PasswordKind,
) -> Result<HashSet<DeviceId>, Error> {
    store::devices_with_kinds(db, devices, &[kind.as_sql()]).await
}

/// Deletes the device's password of `kind` in the caller's connection. Returns true if a
/// password was removed. Takes the per-device lock, as [`store_password`] does, with the same
/// lock order for callers.
///
/// # Errors
///
/// [`Error::Database`] when the store cannot be written.
pub async fn clear_password(
    conn: &mut PgConnection,
    device: DeviceId,
    kind: PasswordKind,
) -> Result<bool, Error> {
    let mut tx = conn.begin().await?;
    store::lock_device(&mut tx, device).await?;
    let removed = store::delete_kinds(&mut tx, device, &[kind.as_sql()]).await?;
    tx.commit().await?;
    Ok(removed > 0)
}
