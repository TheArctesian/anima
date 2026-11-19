//! The ingest tokens of the OwnTracks path (K-E4 Section 11). A token permits only
//! `POST /locations/v1/owntracks`. The hub keeps the SHA-256 of a token and shows the token
//! one time, when it is made; no log line and no audit entry holds it (LOG-007).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sha2::{Digest as _, Sha256};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    Error,
    model::{Bad, IngestToken, token_name},
    store,
};
use crate::ids::IngestTokenId;

/// What starts each ingest token, so that a scanner can find one that leaked.
const PREFIX: &str = "anl_";
const TOKEN_BYTES: usize = 32;

/// A new ingest token with its value, which is shown one time.
pub struct Issued {
    pub token: IngestToken,
    pub value: Zeroizing<String>,
}

fn hash_of(presented: &str) -> [u8; 32] {
    Sha256::digest(presented.as_bytes()).into()
}

/// Makes an ingest token named `name` in the caller's transaction.
///
/// # Errors
/// `Invalid` for a name that is not one line of 1 to 64 characters; `Database`.
///
/// # Panics
/// When the OS gives no random bytes, which a running hub does not see.
pub async fn create_token(
    conn: &mut PgConnection,
    name: &str,
    at: DateTime<Utc>,
) -> Result<Issued, Error> {
    let name = token_name(name).ok_or_else(|| {
        Error::Invalid(vec![Bad::new(
            "name",
            "Give one line of 1 to 64 characters.",
        )])
    })?;
    let mut bytes = Zeroizing::new([0_u8; TOKEN_BYTES]);
    getrandom::fill(&mut *bytes).expect("the OS random source gives a token");
    let value = Zeroizing::new(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(*bytes)));
    let token = IngestToken {
        id: IngestTokenId::from(Uuid::now_v7()),
        name,
        created_at: at,
        last_used_at: None,
        revoked_at: None,
    };
    store::insert_token(conn, &token, &hash_of(&value)).await?;
    Ok(Issued { token, value })
}

/// Ends the token `id` in the caller's transaction; a token that was revoked before keeps its
/// first time.
///
/// # Errors
/// `NotFound`, `Database`.
pub async fn revoke_token(
    conn: &mut PgConnection,
    id: IngestTokenId,
    at: DateTime<Utc>,
) -> Result<IngestToken, Error> {
    store::revoke_token(conn, id, at)
        .await?
        .ok_or(Error::NotFound)
}

/// Every ingest token, newest first, without its value.
///
/// # Errors
/// `Database`.
pub async fn tokens(db: &PgPool) -> Result<Vec<IngestToken>, Error> {
    Ok(store::tokens(db).await?)
}

/// The token that `presented` is, when it is not revoked; its last use becomes `at`.
///
/// # Errors
/// `Database`.
pub async fn admit(
    conn: &mut PgConnection,
    presented: &str,
    at: DateTime<Utc>,
) -> Result<Option<IngestToken>, Error> {
    if !presented.starts_with(PREFIX) {
        return Ok(None);
    }
    Ok(store::use_token(conn, &hash_of(presented), at).await?)
}
