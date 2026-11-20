//! Sealed records: the Class A records that a client seals with the vault key (K-E5 Section 3;
//! VLT-001, VLT-002, VLT-006). The hub stores and returns the bytes and cannot open them: this
//! module has no function that takes a vault key, and it imports no key of the hub.
//!
//! A key offer carries the vault key from a client that has it to a new client, sealed to the
//! key exchange of the two. The hub gives it one time, for 5 minutes.

pub mod graphql;
mod store;
mod trail;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    engine::records::Reference,
    ids::{KeyOfferId, SealedRecordId},
};

/// The most bytes of `sealed` and of `sealed_index`.
pub const SEALED_MAX: usize = 1_048_576;
/// The most bytes of a key offer: one key, sealed to a key exchange.
pub const OFFER_MAX: usize = 4096;
/// The bytes of `key_id`: the SHA-256 of the vault key.
pub const KEY_ID_LEN: usize = 32;
/// How long a key offer can be read.
pub const OFFER_LIFE: TimeDelta = TimeDelta::minutes(5);

/// Why a sealed records operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// A stored row that the checks of the table do not permit.
    #[error("a stored sealed record is not valid")]
    Stored,
}

/// One record as the hub keeps it. It has no `Debug`: the bytes are not for a log.
#[derive(Clone, PartialEq, Eq)]
pub struct Record {
    pub id: SealedRecordId,
    pub owner: Reference,
    pub extension: String,
    pub sealed: Vec<u8>,
    pub sealed_index: Option<Vec<u8>>,
    pub key_id: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What a client stores for one owner record. It has no `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct NewRecord {
    pub owner: Reference,
    pub extension: String,
    pub sealed: Vec<u8>,
    pub sealed_index: Option<Vec<u8>>,
    pub key_id: Vec<u8>,
}

/// True for the key of an extension: a lower-case letter, then at most 39 lower-case letters,
/// digits or underscores.
#[must_use]
pub fn is_extension(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && key.len() <= 40
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The sealed record of `owner`.
///
/// # Errors
/// `Database`, `Stored`.
pub async fn stored(conn: &mut PgConnection, owner: &Reference) -> Result<Option<Record>, Error> {
    store::read(conn, owner).await
}

/// Stores the sealed bytes of `new.owner`: a new record, or new bytes in the record that the
/// owner has. `true` when the record is new.
///
/// # Errors
/// `Database`, `Stored`.
pub async fn keep(
    conn: &mut PgConnection,
    new: &NewRecord,
    now: DateTime<Utc>,
) -> Result<(Record, bool), Error> {
    let id = SealedRecordId::from(Uuid::now_v7());
    store::upsert(conn, id, new, now).await
}

/// Deletes the sealed record of `owner` and gives it; `None` when there is none.
///
/// # Errors
/// `Database`, `Stored`.
pub async fn remove(conn: &mut PgConnection, owner: &Reference) -> Result<Option<Record>, Error> {
    store::delete(conn, owner).await
}

/// Keeps a key offer for [`OFFER_LIFE`] and gives its ID. Each offer that ended is deleted.
///
/// # Errors
/// `Database`.
pub async fn offer(
    conn: &mut PgConnection,
    sealed: &[u8],
    now: DateTime<Utc>,
) -> Result<KeyOfferId, Error> {
    let id = KeyOfferId::from(Uuid::now_v7());
    store::insert_offer(conn, id, sealed, (now, now + OFFER_LIFE)).await?;
    Ok(id)
}

/// Gives the bytes of the offer `id` one time: the read deletes it. `None` for an offer that
/// was read, that ended, or that never was.
///
/// # Errors
/// `Database`.
pub async fn take_offer(
    conn: &mut PgConnection,
    id: KeyOfferId,
    now: DateTime<Utc>,
) -> Result<Option<Vec<u8>>, Error> {
    Ok(store::take_offer(conn, id, now).await?)
}
