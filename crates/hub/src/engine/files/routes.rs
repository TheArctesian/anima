//! The bytes of a file go through `/files/v1`, not through GraphQL (K-E4 Section 9, FIL-005):
//! `POST /files/v1/attachments` streams an upload into the store, and
//! `GET /files/v1/attachments/{id}` streams the plain bytes back, without the location of a
//! JPEG (FIL-003). Each request needs `Authorization: Bearer` with a login token or an API key;
//! an upload needs the write scope. No log line holds a token, a query value or a byte of a file
//! (LOG-007): the route is logged by its template.

mod answer;
mod download;

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Query, Request, State, rejection::QueryRejection},
    http::{
        HeaderMap, StatusCode,
        header::{CONTENT_LENGTH, CONTENT_TYPE},
        request::Parts,
    },
    response::{IntoResponse as _, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::PgPool;

use super::{
    Bad, Error, Store, model, store, trail,
    upload::{Placing, place, stage},
};
use crate::{
    clock::Clock,
    engine::{extensions::Key, records::Reference},
    fleet::{
        access::{self, Identity, Scope},
        events::{Bus, ClientAddr, Operator, Trail},
    },
    ids::ArchiveId,
};
use answer::refused;

/// The key of the engine's own files; every other key is an extension's.
const ENGINE_KEY: &str = "engine";

/// What the two routes need.
#[derive(Clone)]
struct Files {
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    store: Store,
}

/// The routes of `/files/v1`. They have no 1 MiB limit: an upload is counted while it streams
/// and stops at 256 MB (FIL-005).
pub fn router(db: &PgPool, bus: &Bus, clock: &Arc<dyn Clock>, store: &Store) -> Router {
    Router::new()
        .route("/files/v1/attachments", post(upload))
        .route("/files/v1/attachments/{id}", get(download::download))
        .layer(DefaultBodyLimit::disable())
        .with_state(Files {
            db: db.clone(),
            bus: bus.clone(),
            clock: clock.clone(),
            store: store.clone(),
        })
}

/// The operator and the address of a request whose token grants `scope`; else the status of
/// the refusal, with an empty body as the bearer layer gives it.
async fn signed_in(
    files: &Files,
    request: &Parts,
    scope: Scope,
) -> Result<(Operator, ClientAddr), StatusCode> {
    let now = files.clock.now();
    let identity = access::identify(&files.db, &request.headers, None, now).await;
    let identity: Identity = match identity {
        Ok(Some(identity)) => identity,
        Ok(None) => return Err(StatusCode::UNAUTHORIZED),
        Err(error) => {
            error.log();
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };
    let granted = matches!(
        (identity.scope(), scope),
        (Scope::Write, _) | (Scope::Read, Scope::Read)
    );
    let operator = identity.operator().cloned().filter(|_| granted);
    let operator = operator.ok_or(StatusCode::FORBIDDEN)?;
    let client = access::client_addr(&request.extensions);
    Ok((operator, client.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct UploadQuery {
    owner: String,
    name: String,
    key: String,
    archive: Option<String>,
    keep_location: Option<bool>,
}

/// What an upload asks for, checked; or each field to fix.
fn checked(
    query: &UploadQuery,
    headers: &HeaderMap,
) -> Result<(Placing<'static>, String), Vec<Bad>> {
    let mut bad = Vec::new();
    let owner = Reference::parse(&query.owner).ok();
    let name = model::name(&query.name);
    let content_type = headers
        .get(CONTENT_TYPE)
        .map(|value| value.to_str().unwrap_or("?"));
    let media_type = model::media_type(content_type);
    let archive = query
        .archive
        .as_deref()
        .map(str::parse::<ArchiveId>)
        .transpose()
        .ok();
    let key_known = query.key == ENGINE_KEY || Key::parse(&query.key).is_some();
    if owner.is_none() {
        bad.push(Bad::new(
            "owner",
            "Give a record as kind:id, for example book:1234.",
        ));
    }
    if name.is_none() {
        bad.push(Bad::new("name", model::NAME_RULE));
    }
    if media_type.is_none() {
        bad.push(Bad::new(
            "Content-Type",
            "Give a media type such as image/jpeg.",
        ));
    }
    if archive.is_none() {
        bad.push(Bad::new("archive", "Give the ID of an archive."));
    }
    if !key_known {
        bad.push(Bad::new("key", "Give engine or the key of an extension."));
    }
    match (owner, name, media_type, archive) {
        (Some(owner), Some(name), Some(media_type), Some(archive)) if bad.is_empty() => Ok((
            Placing {
                owner,
                name,
                media_type,
                archive,
                by: "",
            },
            query.key.clone(),
        )),
        _ => Err(bad),
    }
}

/// `POST /files/v1/attachments`: the answer is the attachment as JSON, `201`. `413` for a body
/// over 256 MB, with no piece left; `503` with no file store.
async fn upload(
    State(files): State<Files>,
    query: Result<Query<UploadQuery>, QueryRejection>,
    request: Request,
) -> Response {
    let (request, body) = request.into_parts();
    let (operator, client) = match signed_in(&files, &request, Scope::Write).await {
        Ok(signed_in) => signed_in,
        Err(status) => return status.into_response(),
    };
    let headers = &request.headers;
    let declared = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok()?.parse::<u64>().ok());
    if declared.is_some_and(|length| length > model::MAX_FILE) {
        return refused(&Error::TooLarge);
    }
    let Ok(Query(query)) = query else {
        return refused(&Error::Invalid(vec![Bad::new(
            "query",
            "Give owner, name and key.",
        )]));
    };
    let (placing, key_name) = match checked(&query, headers) {
        Ok(checked) => checked,
        Err(bad) => return refused(&Error::Invalid(bad)),
    };
    match stored(
        &files,
        (operator, client),
        (placing, &key_name),
        (query.keep_location.unwrap_or(false), body),
    )
    .await
    {
        Ok(attachment) => {
            (StatusCode::CREATED, Json(answer::attachment(&attachment))).into_response()
        }
        Err(error) => refused(&error),
    }
}

/// Stages the body, then places it and writes its trail in one transaction.
async fn stored(
    files: &Files,
    who: (Operator, ClientAddr),
    (placing, key_name): (Placing<'static>, &str),
    (keep_location, body): (bool, Body),
) -> Result<model::Attachment, Error> {
    if !files.store.available() {
        return Err(Error::Unavailable);
    }
    if let Some(archive) = placing.archive
        && store::archive(&files.db, archive).await?.is_none()
    {
        return Err(Error::Invalid(vec![Bad::new(
            "archive",
            "Give the ID of an archive.",
        )]));
    }
    let key = files.store.keys.key(key_name)?;
    let staged = stage(&files.store, key, body.into_data_stream(), keep_location).await?;
    let by = who.0.id.to_string();
    let trail_of_change = Trail::new(&files.bus, who, files.clock.now());
    let now = trail_of_change.now;
    let mut tx = files.db.begin().await?;
    let placing = Placing { by: &by, ..placing };
    let attachment = place(&mut tx, &files.store, &staged, (placing, now)).await?;
    trail_of_change
        .write(&mut tx, trail::uploaded(&attachment))
        .await?;
    tx.commit().await?;
    Ok(attachment)
}
