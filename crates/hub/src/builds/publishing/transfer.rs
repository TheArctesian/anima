//! The hub's HTTP calls of publishing: a file read back from the bucket itself, the public
//! address's answer for it, and a signed write or delete (BLD-064, BLD-066, D-28). I/O only. A
//! response's length is never trusted: a file is read in chunks and refused at the first byte
//! past its reported size. Nothing here logs; a URL, a header or a response body never reaches
//! an error (LOG-007).

use std::time::Duration;

use minisign_verify::StreamVerifier;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};

use super::{bucket::Signed, files::Received};
use crate::builds::model::UploadedFile;

/// The most of a write's response body the hub reads, to see an error inside a 200.
const MAX_REPLY_BYTES: usize = 64 * 1024;

/// The longest `ETag` the hub keeps; R2 sends a quoted MD5 or a multipart digest.
const MAX_ETAG_BYTES: usize = 128;

/// The longest one write, delete or `HEAD` may take. A try writes only while its hold has at
/// least this long left (D-25).
pub(super) const WRITE_LIMIT: Duration = Duration::from_mins(2);

/// Why a step of publishing stopped.
pub(super) enum Stop {
    /// A check failed: the job fails with this reason (BLD-070).
    Refused(String),
    /// The bucket or the public address could not be reached; the fault word is for the
    /// process log, and the publisher tries again later (D-25).
    Unavailable(&'static str),
    /// A later try took the job, or this try's hold ran out: this try writes nothing more
    /// (D-25).
    TakenOver,
}

/// One file as the bucket holds it, checked: the bytes kept for `SHA256SUMS`, the object's
/// `ETag`, and the verifier fed with every byte.
pub(super) struct Checked<'a> {
    pub(super) kept: Vec<u8>,
    pub(super) etag: String,
    pub(super) verifier: Option<StreamVerifier<'a>>,
}

/// Whether `status` means the far end is busy or down rather than refusing.
fn busy(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

/// `signed` as a request with `body`.
fn request(http: &Client, signed: &Signed, body: Vec<u8>) -> RequestBuilder {
    let method = Method::from_bytes(signed.method.as_bytes()).unwrap_or(Method::GET);
    let mut request = http.request(method, &signed.url).body(body);
    for (name, value) in &signed.headers {
        request = request.header(name, value);
    }
    request
}

/// Reads `file` from the bucket by the signed `GET` `signed` and checks it as it streams
/// (BLD-064, D-28): 200, never more than the reported size, then the reported size and SHA-256,
/// and an `ETag` to copy it by. A `verifier` is fed the same bytes.
pub(super) async fn read<'a>(
    http: &Client,
    signed: Signed,
    file: &UploadedFile,
    mut verifier: Option<StreamVerifier<'a>>,
) -> Result<Checked<'a>, Stop> {
    let mut response = request(http, &signed, Vec::new())
        .send()
        .await
        .map_err(|_| Stop::Unavailable("bucket read"))?;
    let status = response.status();
    if busy(status) {
        return Err(Stop::Unavailable("bucket read"));
    }
    if status != StatusCode::OK {
        return Err(Stop::Refused(format!(
            "{} answered {} in the release bucket, not 200.",
            file.name,
            status.as_u16()
        )));
    }
    let etag = etag_of(&response).ok_or_else(|| {
        Stop::Refused(format!(
            "The release bucket gave no ETag for {}.",
            file.name
        ))
    })?;
    let mut received = Received::new(file);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| Stop::Unavailable("bucket read"))?
    {
        received.push(&chunk).map_err(Stop::Refused)?;
        if let Some(verifier) = &mut verifier {
            verifier.update(&chunk);
        }
    }
    let kept = received.finish().map_err(Stop::Refused)?;
    Ok(Checked {
        kept,
        etag,
        verifier,
    })
}

/// The `ETag` of `response`, when it is short visible ASCII.
fn etag_of(response: &Response) -> Option<String> {
    let value = response
        .headers()
        .get(reqwest::header::ETAG)?
        .to_str()
        .ok()?;
    let fits = !value.is_empty() && value.len() <= MAX_ETAG_BYTES;
    fits.then(|| value.to_owned())
}

/// Checks that people can read `name` at `url`: 200 from that exact URL (BLD-064). The bytes
/// are not read; a cache may still hold an older copy, so they prove nothing (D-28).
pub(super) async fn readable(http: &Client, url: &str, name: &str) -> Result<(), Stop> {
    let response = http
        .get(url)
        .send()
        .await
        .map_err(|_| Stop::Unavailable("public read"))?;
    let status = response.status();
    if busy(status) {
        return Err(Stop::Unavailable("public read"));
    }
    // Defence in depth: the client follows no redirect, and a redirect is still refused here.
    if status != StatusCode::OK || response.url().as_str() != url {
        return Err(Stop::Refused(format!(
            "{name} answered {} at the public address, not 200.",
            status.as_u16()
        )));
    }
    Ok(())
}

/// Sends a signed write or delete with `body`. `what` names the object in a refusal. A 2xx
/// answer whose body is an S3 `<Error>` (as `CopyObject` may send) counts as unavailable; a 412
/// means the copy source changed after the hub checked it (D-28).
pub(super) async fn send(
    http: &Client,
    signed: Signed,
    body: Vec<u8>,
    what: &str,
) -> Result<(), Stop> {
    let mut response = request(http, &signed, body)
        .timeout(WRITE_LIMIT)
        .send()
        .await
        .map_err(|_| Stop::Unavailable("bucket write"))?;
    let status = response.status();
    if busy(status) {
        return Err(Stop::Unavailable("bucket write"));
    }
    if status == StatusCode::PRECONDITION_FAILED {
        return Err(Stop::Refused(format!(
            "The file behind {what} changed in the release bucket after the hub checked it."
        )));
    }
    if !status.is_success() {
        return Err(Stop::Refused(format!(
            "The release bucket refused the write of {what} ({}).",
            status.as_u16()
        )));
    }
    let mut reply = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| Stop::Unavailable("bucket write"))?
    {
        reply.extend_from_slice(&chunk[..chunk.len().min(MAX_REPLY_BYTES - reply.len())]);
        if reply.len() >= MAX_REPLY_BYTES {
            break;
        }
    }
    if reply.windows(7).any(|window| window == b"<Error>") {
        return Err(Stop::Unavailable("bucket write"));
    }
    Ok(())
}

/// The `ETag` the bucket gives `HEAD` `signed` now, or `None` when it has no such object
/// (D-28).
pub(super) async fn etag_now(http: &Client, signed: &Signed) -> Result<Option<String>, Stop> {
    let response = request(http, signed, Vec::new())
        .timeout(WRITE_LIMIT)
        .send()
        .await
        .map_err(|_| Stop::Unavailable("bucket head"))?;
    let status = response.status();
    if busy(status) {
        return Err(Stop::Unavailable("bucket head"));
    }
    Ok((status == StatusCode::OK)
        .then(|| etag_of(&response))
        .flatten())
}
