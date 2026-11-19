//! `POST /locations/v1/owntracks`: the way in of the OwnTracks app (K-E4 Section 11). HTTP
//! Basic: the user is any text and the password is an ingest token. A message of type
//! `location` is stored; every other type is accepted and not stored. The answer is `200`
//! with `[]`, as the app expects. No log line holds a token or a position (LOG-007): the route
//! is logged by its template.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        request::Parts,
    },
    response::{IntoResponse as _, Response},
    routing::post,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::json;
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    Error, Origin, Provider, Seal, admit, audit_closed_hours, count_stored, owntracks,
    store_samples,
};
use crate::{
    clock::Clock,
    engine::sealing_keys::SealingKeys,
    fleet::{access, events::ClientAddr},
};

/// The largest body of one post.
const MAX_BODY: usize = 1024 * 1024;

#[derive(Clone)]
struct Intake {
    db: PgPool,
    clock: Arc<dyn Clock>,
    seal: Seal,
}

/// The route of the OwnTracks path.
pub fn router(db: &PgPool, clock: &Arc<dyn Clock>, keys: &SealingKeys) -> Router {
    Router::new()
        .route("/locations/v1/owntracks", post(owntracks_post))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(Intake {
            db: db.clone(),
            clock: clock.clone(),
            seal: Seal::new(keys.clone()),
        })
}

/// The password of an HTTP Basic header.
fn password(headers: &HeaderMap) -> Option<Zeroizing<String>> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = Zeroizing::new(STANDARD.decode(encoded.trim()).ok()?);
    let text = std::str::from_utf8(&decoded).ok()?;
    let (_, password) = text.split_once(':')?;
    Some(Zeroizing::new(password.to_owned()))
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(
            WWW_AUTHENTICATE,
            HeaderValue::from_static("Basic realm=\"anima locations\""),
        )],
    )
        .into_response()
}

fn failed(error: &Error) -> Response {
    let status = match error {
        Error::Invalid(_) => StatusCode::BAD_REQUEST,
        Error::Key(_) => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        tracing::error!(%error, "a location sample was not stored");
    }
    (status, Json(json!({ "code": status.as_u16() }))).into_response()
}

async fn owntracks_post(State(intake): State<Intake>, request: Parts, body: Bytes) -> Response {
    let Some(presented) = password(&request.headers) else {
        return unauthorized();
    };
    let Some(client) = access::client_addr(&request.extensions) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match taken(&intake, client, &presented, &body).await {
        Ok(true) => (StatusCode::OK, Json(json!([]))).into_response(),
        Ok(false) => unauthorized(),
        Err(error) => failed(&error),
    }
}

/// Stores the sample of `body` for the token `presented`; `false` when no token that is not
/// revoked has this value. A stored fix writes no event and no audit entry of its own: it is
/// counted for its token and its hour, and each hour that ended gets its one audit entry now
/// (R-208).
async fn taken(
    intake: &Intake,
    client: ClientAddr,
    presented: &str,
    body: &[u8],
) -> Result<bool, Error> {
    let now = intake.clock.now();
    let mut tx = intake.db.begin().await?;
    let Some(token) = admit(&mut tx, presented, now).await? else {
        return Ok(false);
    };
    let sample = owntracks::sample(body)
        .map_err(|_| Error::Invalid(vec![super::Bad::new("body", "Give an OwnTracks message.")]))?;
    if let Some(sample) = sample {
        let origin = Origin {
            provider: Provider::OwnTracks,
            account: token.id.to_string(),
        };
        let stored = store_samples(&mut tx, &intake.seal, (&origin, &[sample]), now).await?;
        count_stored(&mut tx, token.id, stored, (now, client)).await?;
    }
    audit_closed_hours(&mut tx, now).await?;
    tx.commit().await?;
    Ok(true)
}
