//! K1 `BuildUploadUrl` and `BuildUploadHeader`: the presigned uploads a builder gets
//! (BLD-060). A URL is a credential for its object; nothing here logs it (LOG-007, BLD-049).

use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

use crate::fleet::builds::UploadUrl;

/// K1 `BuildUploadUrl`.
#[derive(SimpleObject)]
#[graphql(name = "BuildUploadUrl")]
pub(super) struct UploadUrlFields {
    name: String,
    url: String,
    /// Headers the PUT must send exactly as given: Content-Length, Cache-Control and Content-Type.
    headers: Vec<UploadHeaderFields>,
    expires_at: DateTime<Utc>,
}

/// K1 `BuildUploadHeader`.
#[derive(SimpleObject)]
#[graphql(name = "BuildUploadHeader")]
pub(super) struct UploadHeaderFields {
    name: String,
    value: String,
}

impl From<UploadUrl> for UploadUrlFields {
    fn from(upload: UploadUrl) -> Self {
        Self {
            name: upload.name,
            url: upload.url,
            headers: upload
                .headers
                .into_iter()
                .map(|(name, value)| UploadHeaderFields { name, value })
                .collect(),
            expires_at: upload.expires_at,
        }
    }
}
