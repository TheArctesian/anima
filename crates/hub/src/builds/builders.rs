//! Builders: enrolment with a builder token, revocation, the list, and the check of each
//! builder request (BLD-021 to BLD-025, D-1).

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    Error, hidden_text,
    model::{Builder, FieldError, Missing, Platform},
    queue, store,
};
use crate::{
    access::{self, IssuedToken},
    ids::{BuilderId, OperatorId, TokenId},
};

/// The longest builder name, in characters.
const MAX_NAME_CHARS: usize = 64;

/// The refusal of a builder request from another address than the enrolment's (BLD-024).
const WRONG_ADDRESS: &str = "This builder must call from the address it enrolled from.";

/// The refusal of a builder token whose builder record is gone.
pub(super) const NOT_ENROLLED: &str = "This builder is not enrolled.";

/// What the operator's client sends to enrol its computer (BLD-021).
#[derive(Clone, Debug)]
pub struct BuilderInput {
    pub name: String,
    pub platform: Platform,
    /// The VPN address the request came from; the builder must call from it (BLD-024).
    pub client: IpAddr,
}

/// A new builder and its token, shown once. No `Debug`: it holds the token (LOG-007).
pub struct Enrolment {
    pub builder: Builder,
    pub token: IssuedToken,
}

impl Builder {
    /// Whether it reported within the last 60 seconds at `now` (BLD-007).
    #[must_use]
    pub fn online_at(&self, now: DateTime<Utc>) -> bool {
        queue::online(self.last_seen_at, now)
    }
}

/// The name as stored: trimmed, 1 to 64 characters, no control character.
pub(super) fn name(text: &str) -> Result<String, FieldError> {
    let text = text.trim();
    let count = text.chars().count();
    if count == 0
        || count > MAX_NAME_CHARS
        || text
            .chars()
            .any(|c| c.is_control() || hidden_text::is_format(c))
    {
        return Err(FieldError::new(
            "name",
            "Use 1 to 64 characters with no control or format characters.",
        ));
    }
    Ok(text.to_owned())
}

/// Records a builder that calls from the input's address and issues its token, valid 180 days
/// (BLD-021, BLD-022). The caller commits it with its audit entry.
///
/// # Errors
///
/// `Validation` (`name`) for a bad name; `Access` if the token cannot be issued.
pub async fn enrol(
    conn: &mut PgConnection,
    operator: OperatorId,
    input: &BuilderInput,
    now: DateTime<Utc>,
) -> Result<Enrolment, Error> {
    let name = name(&input.name).map_err(|e| Error::Validation(vec![e]))?;
    let token = access::issue_builder_token(conn, operator, now)
        .await
        .map_err(Error::Access)?;
    let builder = Builder {
        id: BuilderId::from(Uuid::now_v7()),
        name,
        platform: input.platform,
        client: input.client,
        enrolled_at: now,
        last_seen_at: None,
    };
    store::insert_builder(conn, &builder, token.id).await?;
    Ok(Enrolment { builder, token })
}

/// Deletes the builder and revokes its token, so it stops working at once (BLD-025). A job it
/// holds goes back to the queue when its lease ends. Returns the builder.
///
/// # Errors
///
/// `NotFound` when there is no such builder; `Access` if the token cannot be revoked.
pub async fn revoke(conn: &mut PgConnection, id: BuilderId) -> Result<Builder, Error> {
    let (builder, token) = store::delete_builder(conn, id)
        .await?
        .ok_or(Error::NotFound(Missing::Builder(id)))?;
    access::revoke_builder_token(conn, token)
        .await
        .map_err(Error::Access)?;
    Ok(builder)
}

/// Every builder, in enrolment order (BLD-025).
///
/// # Errors
///
/// `Database` when the query fails.
pub async fn builders(db: &PgPool) -> Result<Vec<Builder>, Error> {
    store::builders(db).await
}

/// The builder that holds `token`, if `client` is the address it enrolled from (BLD-024).
/// Records that it called at `now`.
///
/// # Errors
///
/// `Forbidden` when the token has no builder or the address differs; `Database` otherwise.
pub async fn builder_for(
    db: &PgPool,
    token: TokenId,
    client: Option<IpAddr>,
    now: DateTime<Utc>,
) -> Result<Builder, Error> {
    let builder = store::builder_by_token(db, token)
        .await?
        .ok_or(Error::Forbidden(NOT_ENROLLED))?;
    if client != Some(builder.client) {
        return Err(Error::Forbidden(WRONG_ADDRESS));
    }
    store::touch(db, builder.id, now).await?;
    Ok(Builder {
        last_seen_at: Some(now),
        ..builder
    })
}
