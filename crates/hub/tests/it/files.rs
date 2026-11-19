//! The file store and the extraction proposals through the hub (K-E4 Sections 9 and 10;
//! FIL-001 to FIL-005, VIS-001 to VIS-004, LOG-007).

mod archives;
mod keys;
mod photo;
mod proposals;
mod upload;

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anima_hub::{
    clock::{Clock, FixedClock},
    engine::{
        files::{Derived, Store},
        sealing_keys::SealingKeys,
    },
    fleet::events::Bus,
    router,
    schema::{Dependencies, build_schema},
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{HeaderMap, Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use http_body_util::BodyExt as _;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::hub::{CLIENT, Hub, dependencies, set_operator, start};

const BOOK: &str = "book:isbn-9780141439518";

/// A hub with a file store in a folder of its own, its data keys in another, and the
/// `/files/v1` routes beside `/graphql`.
struct FileHub {
    app: Router,
    token: String,
    files: tempfile::TempDir,
    keys: tempfile::TempDir,
}

impl FileHub {
    async fn new(pool: &PgPool) -> Self {
        Self::deriving(pool, Vec::new()).await
    }

    async fn deriving(pool: &PgPool, derived: Vec<Arc<dyn Derived>>) -> Self {
        let (files, keys) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let data_keys = SealingKeys::open(Some(keys.path()), b"credential").unwrap();
        let store = Store::in_folder(Some(files.path()), data_keys).deriving(derived);
        Self::with_store(pool, store, (files, keys)).await
    }

    async fn with_store(
        pool: &PgPool,
        store: Store,
        (files, keys): (tempfile::TempDir, tempfile::TempDir),
    ) -> Self {
        set_operator(pool).await;
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
        let bus = Bus::new(64);
        let parts = Dependencies {
            files: store,
            ..dependencies(pool, clock.clone(), bus.clone())
        };
        let routes = anima_hub::files(&parts);
        let app = router(build_schema(parts), pool.clone(), clock).merge(routes);
        let token = Hub::new(pool, start()).token().await;
        Self {
            app,
            token,
            files,
            keys,
        }
    }

    /// Sends `request` from the test client.
    async fn send(&self, mut request: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
        let peer: SocketAddr = format!("{CLIENT}:50000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = self.app.clone().oneshot(request).await.unwrap();
        let (status, headers) = (response.status(), response.headers().clone());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, headers, bytes.to_vec())
    }

    /// Uploads `body` to `owner` as `query` says, with `token`.
    async fn upload_as(&self, token: &str, query: &str, body: Body) -> (StatusCode, Value) {
        let request = Request::post(format!("/files/v1/attachments?{query}"))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "image/jpeg")
            .body(body)
            .unwrap();
        let (status, _, bytes) = self.send(request).await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn upload(&self, query: &str, bytes: &[u8]) -> Value {
        let (status, answer) = self
            .upload_as(&self.token, query, Body::from(bytes.to_vec()))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{answer}");
        answer
    }

    async fn download(&self, id: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
        let request = Request::get(format!("/files/v1/attachments/{id}"))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .body(Body::empty())
            .unwrap();
        self.send(request).await
    }

    /// A download whose body may end with an error: the status and the bytes, or `None` when
    /// the body did not arrive whole.
    async fn try_download(&self, id: &str) -> Option<(StatusCode, Vec<u8>)> {
        let mut request = Request::get(format!("/files/v1/attachments/{id}"))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .body(Body::empty())
            .unwrap();
        let peer: SocketAddr = format!("{CLIENT}:50000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.ok()?;
        Some((status, body.to_bytes().to_vec()))
    }

    /// Runs a GraphQL request on the same app.
    async fn post(&self, query: &str, variables: Value) -> Value {
        let body = json!({ "query": query, "variables": variables }).to_string();
        let request = Request::post("/graphql")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .body(Body::from(body))
            .unwrap();
        let (status, _, bytes) = self.send(request).await;
        assert!(status.is_success(), "{status}");
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Every regular file below the file store, staging copies included.
    fn stored_files(&self) -> Vec<PathBuf> {
        files_below(self.files.path())
    }
}

fn files_below(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files_below(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn sha256(bytes: &[u8]) -> String {
    anima_hub::engine::sealing_keys::hex(&Sha256::digest(bytes))
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("select count(*) from files.{table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

/// An API key of the read scope for the fixture operator.
async fn read_key(pool: &PgPool) -> String {
    let bytes = [9_u8; 32];
    sqlx::query(
        "insert into access.api_tokens (id, operator_id, token_hash, kind, name, display_prefix, \
         scope, expires_at, created_at, made_for) select gen_random_uuid(), id, $1, 'api_key', \
         'reader', 'anima_CQkJCQ', 'read', $2, $3, 'user' from access.operators",
    )
    .bind(Sha256::digest(bytes).to_vec())
    .bind(start() + chrono::TimeDelta::days(30))
    .bind(start() - chrono::TimeDelta::days(1))
    .execute(pool)
    .await
    .unwrap();
    format!("anima_{}", URL_SAFE_NO_PAD.encode(bytes))
}
