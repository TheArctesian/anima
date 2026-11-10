//! An in-process stand-in for R2 (D-24): the S3 API on one listener and the public address on
//! another. The API checks every request's Signature Version 4 itself (`signing.rs`), so a URL
//! replayed for another object, a wrong length or header, an expired URL or a skewed date fails
//! as it would at R2. It serves `PUT`, `GET` with an `ETag`, `CopyObject` with
//! `x-amz-copy-source-if-match`, and `DELETE`. Faults are set per test.
//!
//! The state sits behind a `std` mutex that no handler holds across an `.await`.

use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use sha2::{Digest as _, Sha256};

pub use super::signing::hex;
use super::signing::{text, verify};

pub const BUCKET: &str = "anima-releases";
pub const KEY_ID: &str = "AKIDFAKER2TEST";
pub const SECRET: &str = "fake-r2-secret-for-tests-only";

/// One stored object.
#[derive(Clone, Debug, Default)]
pub struct Object {
    pub bytes: Vec<u8>,
    pub cache_control: Option<String>,
    pub content_type: Option<String>,
}

impl Object {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
            ..Self::default()
        }
    }

    /// As R2 gives it: quoted, the same for the same bytes.
    pub fn etag(&self) -> String {
        format!("\"{}\"", &hex(&Sha256::digest(&self.bytes))[..32])
    }
}

/// What the fake holds and what the test sets.
#[derive(Default)]
pub struct Store {
    /// By key, without the bucket.
    pub objects: BTreeMap<String, Object>,
    /// The keys written, in order.
    pub writes: Vec<String>,
    /// The keys deleted, in order.
    pub deletes: Vec<String>,
    /// Why each refused request was refused.
    pub refused: Vec<String>,
    /// The fake's clock for expiry and skew; `None` takes the request's own date.
    pub now: Option<DateTime<Utc>>,
    /// A key whose reads come back with their last byte changed.
    pub tamper: Option<String>,
    /// A key whose reads come back one byte longer.
    pub longer: Option<String>,
    /// A key whose write waits for `gate` before it lands.
    pub hang: Option<String>,
    /// A key whose write lands, then waits for `gate` before it answers.
    pub hang_answer: Option<String>,
    /// A key whose API read waits for `gate`.
    pub hang_read: Option<String>,
    /// Opens the requests that wait; `waiting` counts them.
    pub gate: Arc<tokio::sync::Notify>,
    pub waiting: usize,
    /// A key whose write lands, then answers 503: the answer is lost.
    pub lose: Option<String>,
    /// A key the public address answers with these older bytes, as a warm cache would.
    pub stale: Option<(String, Vec<u8>)>,
    /// A key the public address answers 404 for.
    pub public_missing: Option<String>,
    /// A key whose API read comes back without an `ETag`.
    pub no_etag: Option<String>,
    /// Every API call answers 503.
    pub unavailable: bool,
}

pub type Shared = Arc<Mutex<Store>>;

/// The two listeners and the shared store.
pub struct FakeR2 {
    pub api: SocketAddr,
    pub public: SocketAddr,
    pub store: Shared,
}

impl FakeR2 {
    pub async fn start() -> Self {
        let store = Shared::default();
        let api = serve(Router::new().fallback(api).with_state(store.clone())).await;
        let public = serve(Router::new().fallback(public).with_state(store.clone())).await;
        Self { api, public, store }
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}", self.api)
    }

    pub fn public_base(&self) -> String {
        format!("http://{}", self.public)
    }

    pub fn object(&self, key: &str) -> Option<Object> {
        self.store.lock().unwrap().objects.get(key).cloned()
    }

    pub fn with<T>(&self, change: impl FnOnce(&mut Store) -> T) -> T {
        change(&mut self.store.lock().unwrap())
    }
}

async fn serve(router: Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    addr
}

/// The object at `key` as a read gives it, with the tamper and length faults.
fn served(store: &Store, key: &str) -> Option<Object> {
    let mut object = store.objects.get(key).cloned()?;
    if store.tamper.as_deref() == Some(key)
        && let Some(last) = object.bytes.last_mut()
    {
        *last ^= 1;
    }
    if store.longer.as_deref() == Some(key) {
        object.bytes.push(b'\n');
    }
    Some(object)
}

/// Waits at the gate when `pick` names `key`.
async fn hold(store: &Shared, key: &str, pick: fn(&Store) -> Option<&String>) {
    let gate = {
        let store = store.lock().unwrap();
        if pick(&store).map(String::as_str) != Some(key) {
            return;
        }
        store.gate.clone()
    };
    let opened = gate.notified();
    store.lock().unwrap().waiting += 1;
    opened.await;
}

async fn public(State(store): State<Shared>, uri: Uri) -> Response {
    let key = uri.path().trim_start_matches('/').to_owned();
    let object = {
        let store = store.lock().unwrap();
        if store.public_missing.as_deref() == Some(key.as_str()) {
            None
        } else {
            match &store.stale {
                Some((stale, bytes)) if *stale == key => Some(Object::new(bytes.clone())),
                _ => served(&store, &key),
            }
        }
    };
    let Some(object) = object else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let cache = object.cache_control.unwrap_or_default();
    ([(header::CACHE_CONTROL, cache)], object.bytes).into_response()
}

async fn api(State(store): State<Shared>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let Ok(body) = axum::body::to_bytes(body, 1 << 24).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let (now, unavailable) = {
        let store = store.lock().unwrap();
        (store.now, store.unavailable)
    };
    if unavailable {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let Some(key) = parts
        .uri
        .path()
        .strip_prefix(&format!("/{BUCKET}/"))
        .map(str::to_owned)
    else {
        return refuse(&store, "not this bucket");
    };
    if let Err(why) = verify(&parts, &body, now) {
        return refuse(&store, &why);
    }
    match parts.method {
        Method::GET => read(&store, &key).await,
        Method::PUT => write(&store, &key, &parts.headers, body.to_vec()).await,
        Method::DELETE => {
            let mut store = store.lock().unwrap();
            store.objects.remove(&key);
            store.deletes.push(key);
            StatusCode::NO_CONTENT.into_response()
        }
        _ => refuse(&store, "method"),
    }
}

async fn read(store: &Shared, key: &str) -> Response {
    hold(store, key, |store| store.hang_read.as_ref()).await;
    let Some(object) = served(&store.lock().unwrap(), key) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let store = store.lock().unwrap();
    if store.no_etag.as_deref() == Some(key) {
        return object.bytes.into_response();
    }
    let stored = store.objects.get(key).map(Object::etag);
    ([(header::ETAG, stored.unwrap_or_default())], object.bytes).into_response()
}

/// A `PUT`, or a `CopyObject` when it names a source; 412 when the source's `ETag` is not the
/// one the request names.
async fn write(store: &Shared, key: &str, headers: &HeaderMap, body: Vec<u8>) -> Response {
    hold(store, key, |store| store.hang.as_ref()).await;
    let object = {
        let mut store = store.lock().unwrap();
        let bytes = match text(headers, "x-amz-copy-source") {
            Some(source) => {
                let from = source
                    .strip_prefix(&format!("/{BUCKET}/"))
                    .unwrap_or(source);
                let Some(object) = store.objects.get(from) else {
                    return StatusCode::NOT_FOUND.into_response();
                };
                let wanted = text(headers, "x-amz-copy-source-if-match");
                if wanted.is_some_and(|etag| etag != object.etag()) {
                    return StatusCode::PRECONDITION_FAILED.into_response();
                }
                object.bytes.clone()
            }
            None => body,
        };
        let object = Object {
            bytes,
            cache_control: text(headers, "cache-control").map(str::to_owned),
            content_type: text(headers, "content-type").map(str::to_owned),
        };
        store.objects.insert(key.to_owned(), object.clone());
        store.writes.push(key.to_owned());
        object
    };
    hold(store, key, |store| store.hang_answer.as_ref()).await;
    if store.lock().unwrap().lose.as_deref() == Some(key) {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    ([(header::ETAG, object.etag())], "").into_response()
}

fn refuse(store: &Shared, why: &str) -> Response {
    store.lock().unwrap().refused.push(why.to_owned());
    (
        StatusCode::FORBIDDEN,
        "<Error><Code>SignatureDoesNotMatch</Code></Error>",
    )
        .into_response()
}
