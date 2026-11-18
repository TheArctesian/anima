//! The JSON answers of `/files/v1`: the attachment, and a refusal with its K7 code. A refusal
//! names the field to fix, never the value that was sent (LOG-007).

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use serde_json::{Value, json};

use crate::engine::files::{Error, model::Attachment, pieces::StoreError};

/// The attachment as the upload answers it.
pub(super) fn attachment(attachment: &Attachment) -> Value {
    json!({
        "id": attachment.id.to_string(),
        "owner": attachment.owner.as_str(),
        "hash": attachment.hash,
        "size": attachment.size,
        "name": attachment.name,
        "mediaType": attachment.media_type,
        "key": attachment.key_name,
        "archiveId": attachment.archive.map(|id| id.to_string()),
        "latitude": attachment.position.map(|p| p.latitude),
        "longitude": attachment.position.map(|p| p.longitude),
        "createdAt": attachment.created_at,
    })
}

/// `{ "code": …, "message": … }` with `status`.
pub(super) fn coded(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(json!({ "code": code, "message": message }))).into_response()
}

/// The answer to a failed request. A fault of the hub is logged by its fixed text, which holds
/// no value, and answered `INTERNAL` with no detail (API-010).
pub(super) fn refused(error: &Error) -> Response {
    match error {
        Error::Invalid(bad) => {
            let fields: Vec<Value> = bad
                .iter()
                .map(|bad| json!({ "field": bad.field, "message": bad.message }))
                .collect();
            let body = json!({ "code": "VALIDATION", "message": "Fix the fields named.", "fields": fields });
            (StatusCode::BAD_REQUEST, Json(body)).into_response()
        }
        Error::TooLarge => coded(
            StatusCode::PAYLOAD_TOO_LARGE,
            "VALIDATION",
            "A file is at most 256 MB.",
        ),
        Error::Body => coded(
            StatusCode::BAD_REQUEST,
            "VALIDATION",
            "The file did not arrive whole.",
        ),
        Error::NotFound("attachment") | Error::Store(StoreError::NotFound) => coded(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "No attachment has this ID.",
        ),
        Error::Unavailable | Error::Key(crate::engine::data_keys::DataKeyError::Unavailable) => {
            coded(
                StatusCode::SERVICE_UNAVAILABLE,
                "UNAVAILABLE",
                "This hub has no file store: set ANIMA_FILES_DIR and ANIMA_DATA_KEY_DIR.",
            )
        }
        _ => {
            tracing::error!(error = %error, "a file request failed");
            coded(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL",
                "The hub could not do this.",
            )
        }
    }
}
