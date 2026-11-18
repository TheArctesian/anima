//! `GET /files/v1/attachments/{id}`: the plain bytes as a stream, with the stored media type,
//! as an attachment. A file that starts as a JPEG is sent without its location metadata, and
//! the stored piece does not change (FIL-003). The download writes one audit entry and no event.

use std::{io, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{
        HeaderValue, StatusCode,
        header::{
            CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_SECURITY_POLICY, CONTENT_TYPE,
            X_CONTENT_TYPE_OPTIONS,
        },
        request::Parts,
    },
    response::{IntoResponse as _, Response},
};
use bytes::Bytes;
use futures_util::{StreamExt as _, stream};

use super::{Files, answer::refused, signed_in};
use crate::{
    engine::files::{
        Error, PlainStream,
        location::{Cleaner, is_jpeg},
        model::Attachment,
        parts::opened,
        store, trail,
    },
    fleet::{
        access::Scope,
        events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
    },
    ids::AttachmentId,
};

/// The header that says whether the hub removed the location of the copy (FIL-003).
pub const LOCATION_REMOVED: &str = "anima-location-removed";

pub(super) async fn download(
    State(files): State<Files>,
    Path(id): Path<String>,
    request: Parts,
) -> Response {
    let who = match signed_in(&files, &request, Scope::Read).await {
        Ok(who) => who,
        Err(status) => return status.into_response(),
    };
    let Ok(id) = id.parse::<AttachmentId>() else {
        return refused(&Error::NotFound("attachment"));
    };
    match sent(&files, who, id).await {
        Ok((attachment, body, cleaned)) => response(&attachment, body, cleaned),
        Err(error) => refused(&error),
    }
}

/// The attachment, its plain bytes and whether they are cleaned; the audit entry is written
/// before the first byte goes.
async fn sent(
    files: &Files,
    (operator, client): (Operator, ClientAddr),
    id: AttachmentId,
) -> Result<(Attachment, Body, bool), Error> {
    let attachment = store::attachment(&files.db, id)
        .await?
        .ok_or(Error::NotFound("attachment"))?;
    let pieces = files.store.pieces()?;
    let key = Arc::new(files.store.keys.key(&attachment.key_name)?);
    let sealed = pieces.get(&attachment.key_name, &attachment.hash).await?;
    let mut plain = opened(sealed, key, attachment.hash.clone());
    let Some(Ok(first)) = plain.next().await else {
        return Err(Error::Damaged);
    };
    let cleaned = is_jpeg(&first);
    let mut tx = files.db.begin().await?;
    let entry = NewAudit {
        operator,
        action: AuditAction::Download,
        target: AuditTarget::Attachment(attachment.id),
        client,
        change: trail::file_change(&attachment),
    };
    events::audit(&mut tx, files.clock.now(), entry).await?;
    tx.commit().await?;
    let body = if cleaned {
        Body::from_stream(without_location(first, plain))
    } else {
        Body::from_stream(stream::once(async { Ok::<_, io::Error>(first) }).chain(plain))
    };
    Ok((attachment, body, cleaned))
}

/// The JPEG of `first` and `rest` without its location metadata. A file whose segments are
/// not those of a JPEG ends with an error after the bytes before them, never with them.
fn without_location(first: Bytes, rest: PlainStream) -> PlainStream {
    let chunks = stream::once(async { Ok(first) }).chain(rest);
    Box::pin(chunks.scan(Cleaner::new(), |cleaner, chunk| {
        let cleaned = chunk.and_then(|chunk| {
            cleaner
                .feed(&chunk)
                .map(Bytes::from)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        });
        std::future::ready(Some(cleaned))
    }))
}

fn response(attachment: &Attachment, body: Body, cleaned: bool) -> Response {
    let media_type = HeaderValue::from_str(&attachment.media_type)
        .unwrap_or(HeaderValue::from_static("application/octet-stream"));
    let disposition = HeaderValue::from_str(&disposition(&attachment.name))
        .unwrap_or(HeaderValue::from_static("attachment"));
    let removed = HeaderValue::from_static(if cleaned { "true" } else { "false" });
    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, media_type),
            (CONTENT_DISPOSITION, disposition),
            (X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
            (
                CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("default-src 'none'; sandbox"),
            ),
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (LOCATION_REMOVED.parse().expect("a header name"), removed),
        ],
        body,
    )
        .into_response()
}

/// `attachment; filename*=UTF-8''<name>`, each byte outside the RFC 5987 attribute characters
/// percent-encoded, so that no name can end the header or add one.
fn disposition(name: &str) -> String {
    let mut encoded = String::with_capacity(name.len() * 3);
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&super::super::hex(&[byte]).to_ascii_uppercase());
        }
    }
    format!("attachment; filename*=UTF-8''{encoded}")
}
