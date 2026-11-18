//! One link of the chain and its hash (LOG-021 to LOG-023): pure, no clock and no database.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::canonical::canonical;

/// A SHA-256 hash.
pub(in crate::fleet::events) type Hash = [u8; 32];

/// The previous hash of link 1 (LOG-023).
pub(in crate::fleet::events) const FIRST_PREVIOUS: Hash = [0; 32];

/// An audit entry as the database stores it: the values the hash covers (LOG-022).
#[derive(Clone, Debug, PartialEq)]
pub(in crate::fleet::events) struct StoredEntry {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    pub operator_id: Uuid,
    pub operator_name: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub client: IpAddr,
    pub change: Value,
    pub actor_kind: Option<String>,
    pub token_kind: Option<String>,
    pub api_key_name: Option<String>,
}

/// One stored link: its place, its entry and its two hashes (LOG-021, LOG-025).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::events) struct Link {
    pub seq: i64,
    pub entry_id: Uuid,
    pub entry_at: DateTime<Utc>,
    pub previous: Hash,
    pub hash: Hash,
}

impl Link {
    /// The link of `entry` at `seq`, after the link whose hash is `previous`.
    pub(in crate::fleet::events) fn new(seq: i64, previous: Hash, entry: &StoredEntry) -> Self {
        Self {
            seq,
            entry_id: entry.id,
            entry_at: entry.at,
            previous,
            hash: hash(seq, &previous, entry),
        }
    }
}

/// SHA-256 of the canonical JSON of the 14 members of LOG-022.
pub(in crate::fleet::events) fn hash(seq: i64, previous: &Hash, entry: &StoredEntry) -> Hash {
    let object = json!({
        "seq": seq,
        "previous": hex(previous),
        "id": entry.id.to_string(),
        "at": entry.at.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string(),
        "operatorId": entry.operator_id.to_string(),
        "operatorName": entry.operator_name,
        "action": entry.action,
        "targetType": entry.target_type,
        "targetId": entry.target_id,
        "clientAddr": entry.client.to_string(),
        "change": entry.change,
        "actorKind": entry.actor_kind,
        "tokenKind": entry.token_kind,
        "apiKeyName": entry.api_key_name,
    });
    Sha256::digest(canonical(&object).as_bytes()).into()
}

/// `hash` as 64 lower-case hexadecimal characters.
pub(in crate::fleet::events) fn hex(hash: &Hash) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}
