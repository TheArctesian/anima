//! The sealed records GraphQL part (K1; K-E5 Section 3): the bytes that a client sealed, as
//! base64, and the carried offer of the vault key. No field takes a vault key (VLT-001).
//!
//! Each change writes its row, one audit entry and one event in one transaction. An entry never
//! holds a byte of a record.

use std::sync::Arc;

use async_graphql::{Context, ID, InputObject, Object, Result, SimpleObject};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};

use super::{
    Error, KEY_ID_LEN, NewRecord, OFFER_MAX, Record, SEALED_MAX, is_extension, keep, offer, remove,
    stored, take_offer, trail,
};
use crate::{
    api_errors::schema_dependency,
    api_errors::{FieldError, begin, commit, internal, invalid, pool},
    clock::Clock,
    engine::records::Reference,
    fleet::{access::Authenticated, events::Trail},
    ids::KeyOfferId,
};

const BAD_OWNER: &str = "Give a record as kind:id, for example passport:1234.";
const BAD_BYTES: &str = "Give 1 byte to 1 MiB of sealed bytes as base64.";

/// K1 `SealedRecord`.
#[derive(SimpleObject)]
#[graphql(name = "SealedRecord")]
struct RecordView {
    id: ID,
    /// The record that the bytes belong to, as `kind:id`. One owner has one sealed record.
    owner: String,
    /// The key of the extension that owns the record.
    extension: String,
    /// What the client sealed with the vault key, as base64. The hub cannot open it.
    sealed: String,
    /// The search index that the client sealed, as base64, or null.
    sealed_index: Option<String>,
    /// The SHA-256 of the vault key that sealed the record, as base64.
    key_id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<Record> for RecordView {
    fn from(record: Record) -> Self {
        Self {
            id: ID(record.id.to_string()),
            owner: record.owner.to_string(),
            extension: record.extension,
            sealed: STANDARD.encode(record.sealed),
            sealed_index: record.sealed_index.map(|index| STANDARD.encode(index)),
            key_id: STANDARD.encode(record.key_id),
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// K1 `SealedRecordInput`.
#[derive(InputObject)]
#[graphql(name = "SealedRecordInput")]
struct RecordInput {
    /// The record that the bytes belong to, as `kind:id`.
    owner: String,
    /// The key of the extension that owns the record.
    extension: String,
    /// The sealed bytes as base64: 1 byte to 1 MiB.
    sealed: String,
    /// The sealed search index as base64, at most 1 MiB.
    sealed_index: Option<String>,
    /// The SHA-256 of the vault key as base64: 32 bytes. It is not the key.
    key_id: String,
}

/// The bytes of `text` when it is base64 of 1 to `max` bytes.
fn bytes(text: &str, max: usize) -> Option<Vec<u8>> {
    // 4 characters of base64 hold 3 bytes: a longer text is not decoded.
    if text.len() > max.div_ceil(3) * 4 {
        return None;
    }
    STANDARD
        .decode(text)
        .ok()
        .filter(|decoded| !decoded.is_empty() && decoded.len() <= max)
}

fn owner_of(text: &str) -> Result<Reference> {
    Reference::parse(text).map_err(|_| invalid(&[FieldError::new("owner", BAD_OWNER)]))
}

/// The record the input asks for, or each field to fix. A message never quotes the input.
fn new_record(input: &RecordInput) -> Result<NewRecord, Vec<FieldError>> {
    let mut bad = Vec::new();
    let owner = Reference::parse(&input.owner);
    if owner.is_err() {
        bad.push(FieldError::new("owner", BAD_OWNER));
    }
    if !is_extension(&input.extension) {
        bad.push(FieldError::new(
            "extension",
            "Give the key of an extension: lower-case letters, digits or underscores.",
        ));
    }
    let sealed = bytes(&input.sealed, SEALED_MAX);
    if sealed.is_none() {
        bad.push(FieldError::new("sealed", BAD_BYTES));
    }
    let index = input
        .sealed_index
        .as_deref()
        .map(|text| bytes(text, SEALED_MAX));
    if matches!(index, Some(None)) {
        bad.push(FieldError::new("sealedIndex", BAD_BYTES));
    }
    let key_id = bytes(&input.key_id, KEY_ID_LEN).filter(|id| id.len() == KEY_ID_LEN);
    if key_id.is_none() {
        bad.push(FieldError::new(
            "keyId",
            "Give the SHA-256 of the vault key as base64: 32 bytes.",
        ));
    }
    match (owner, sealed, key_id) {
        (Ok(owner), Some(sealed), Some(key_id)) if bad.is_empty() => Ok(NewRecord {
            owner,
            extension: input.extension.clone(),
            sealed,
            sealed_index: index.flatten(),
            key_id,
        }),
        _ => Err(bad),
    }
}

fn failed(error: Error) -> async_graphql::Error {
    internal(error)
}

#[derive(Default)]
pub struct Query;

#[Object(name = "SealedRecordsQuery")]
impl Query {
    /// The sealed record of `owner` (`kind:id`), or null. The bytes are as the client sealed
    /// them; the hub cannot open them.
    #[graphql(guard = "Authenticated")]
    async fn sealed_record(&self, ctx: &Context<'_>, owner: String) -> Result<Option<RecordView>> {
        let owner = owner_of(&owner)?;
        let mut conn = pool(ctx)?.acquire().await.map_err(|e| failed(e.into()))?;
        let found = stored(&mut conn, &owner).await.map_err(failed)?;
        Ok(found.map(Into::into))
    }

    /// The sealed bytes of a vault key offer, as base64, one time: the read deletes the offer.
    /// Null for an offer that was read, that is older than 5 minutes, or that never was.
    #[graphql(guard = "Authenticated")]
    async fn vault_key_offer(&self, ctx: &Context<'_>, id: ID) -> Result<Option<String>> {
        let Ok(id) = id.parse::<KeyOfferId>() else {
            return Ok(None);
        };
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let mut conn = pool(ctx)?.acquire().await.map_err(|e| failed(e.into()))?;
        let taken = take_offer(&mut conn, id, now).await.map_err(failed)?;
        Ok(taken.map(|sealed| STANDARD.encode(sealed)))
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "SealedRecordsMutation")]
impl Mutation {
    /// Stores the bytes that a client sealed for one owner record. A second call for the same
    /// owner replaces the bytes. `VALIDATION` names each field to fix.
    #[graphql(guard = "Authenticated")]
    async fn store_sealed_record(
        &self,
        ctx: &Context<'_>,
        input: RecordInput,
    ) -> Result<RecordView> {
        let new = new_record(&input).map_err(|bad| invalid(&bad))?;
        let trail_of_change = Trail::from_context(ctx)?;
        let now = trail_of_change.now;
        let mut tx = begin(ctx).await?;
        let (record, made) = keep(&mut tx, &new, now).await.map_err(failed)?;
        trail_of_change
            .write(&mut tx, trail::kept(&record, made))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(record.into())
    }

    /// Deletes the sealed record of `owner`. `true` when there was one.
    #[graphql(guard = "Authenticated")]
    async fn delete_sealed_record(&self, ctx: &Context<'_>, owner: String) -> Result<bool> {
        let owner = owner_of(&owner)?;
        let trail_of_change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let Some(record) = remove(&mut tx, &owner).await.map_err(failed)? else {
            return Ok(false);
        };
        trail_of_change
            .write(&mut tx, trail::removed(&record))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(true)
    }

    /// Keeps a vault key that one client sealed to its key exchange with a new client, as
    /// base64 of at most 4096 bytes, and gives the ID of the offer. The offer is read one time
    /// in the next 5 minutes. The hub cannot open it.
    #[graphql(guard = "Authenticated")]
    async fn offer_vault_key(&self, ctx: &Context<'_>, sealed: String) -> Result<ID> {
        let sealed = bytes(&sealed, OFFER_MAX).ok_or_else(|| {
            invalid(&[FieldError::new(
                "sealed",
                "Give 1 to 4096 sealed bytes as base64.",
            )])
        })?;
        let trail_of_change = Trail::from_context(ctx)?;
        let now = trail_of_change.now;
        let mut tx = begin(ctx).await?;
        let id = offer(&mut tx, &sealed, now).await.map_err(failed)?;
        trail_of_change
            .write(&mut tx, trail::offered(id, sealed.len()))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(ID(id.to_string()))
    }
}
