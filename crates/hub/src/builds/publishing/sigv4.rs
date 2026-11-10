//! AWS Signature Version 4, as R2's S3 API checks it (D-24): presigned URLs for the builder and
//! the `Authorization` header for the hub's own requests. Pure: the time, the key and the request
//! come in as parameters (CQ-021). Nothing here logs; a signature, a signing key or a string to
//! sign never leaves this module except as the finished query or header (LOG-007).

use std::fmt::Write as _;

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

const ALGORITHM: &str = "AWS4-HMAC-SHA256";

/// The payload hash of a presigned request: the body is not part of the signature.
const UNSIGNED_PAYLOAD: &str = "UNSIGNED-PAYLOAD";

/// Who signs, and for which region and service.
pub(super) struct Key<'a> {
    pub(super) id: &'a str,
    pub(super) secret: &'a str,
    pub(super) region: &'a str,
    pub(super) service: &'a str,
}

/// One request to sign. The path and the query are not encoded yet; `headers` holds every
/// header that is signed except `host`, which comes from `host`.
pub(super) struct Request<'a> {
    pub(super) method: &'a str,
    /// `host` or `host:port`, exactly as the `Host` header carries it.
    pub(super) host: &'a str,
    /// Starts with `/`.
    pub(super) path: &'a str,
    pub(super) query: Vec<(String, String)>,
    pub(super) headers: Vec<(String, String)>,
}

/// Lower-case hex of `bytes`.
pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// Lower-case hex of the SHA-256 of `bytes`.
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Signature Version 4 URI encoding: unreserved characters stay, every other byte is `%XX` in upper case; `/`
/// stays only when `keep_slash` (a path).
pub(super) fn encode(text: &str, keep_slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if unreserved || (keep_slash && byte == b'/') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn hmac(key: &[u8], data: &str) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes a key of any length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().into()
}

/// The key of one day, region and service, derived from the secret.
pub(super) fn signing_key(
    secret: &str,
    date: &str,
    region: &str,
    service: &str,
) -> Zeroizing<[u8; 32]> {
    let first = Zeroizing::new(format!("AWS4{secret}"));
    let day = Zeroizing::new(hmac(first.as_bytes(), date));
    let region = Zeroizing::new(hmac(&*day, region));
    let service = Zeroizing::new(hmac(&*region, service));
    Zeroizing::new(hmac(&*service, "aws4_request"))
}

/// `YYYYMMDDTHHMMSSZ`.
pub(super) fn amz_date(now: DateTime<Utc>) -> String {
    now.format("%Y%m%dT%H%M%SZ").to_string()
}

fn scope(key: &Key<'_>, now: DateTime<Utc>) -> String {
    format!(
        "{}/{}/{}/aws4_request",
        now.format("%Y%m%d"),
        key.region,
        key.service
    )
}

/// The canonical headers and the signed header list: names in lower case, sorted, values
/// trimmed with inner runs of spaces made one.
fn canonical_headers(request: &Request<'_>) -> (String, String) {
    let mut headers: Vec<(String, String)> = request
        .headers
        .iter()
        .map(|(name, value)| {
            let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
            (name.to_ascii_lowercase(), value)
        })
        .chain([("host".to_owned(), request.host.to_owned())])
        .collect();
    headers.sort();
    let lines = headers
        .iter()
        .fold(String::new(), |mut lines, (name, value)| {
            let _ = writeln!(lines, "{name}:{value}");
            lines
        });
    let names: Vec<&str> = headers.iter().map(|(name, _)| name.as_str()).collect();
    (lines, names.join(";"))
}

/// The query, each name and value encoded, sorted by name then value.
pub(super) fn canonical_query(query: &[(String, String)]) -> String {
    let mut pairs: Vec<(String, String)> = query
        .iter()
        .map(|(name, value)| (encode(name, false), encode(value, false)))
        .collect();
    pairs.sort();
    let pairs: Vec<String> = pairs
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    pairs.join("&")
}

/// The signature of `request` with `payload` as its payload hash.
fn signature(key: &Key<'_>, request: &Request<'_>, payload: &str, now: DateTime<Utc>) -> String {
    let (headers, signed) = canonical_headers(request);
    let canonical = format!(
        "{}\n{}\n{}\n{headers}\n{signed}\n{payload}",
        request.method,
        encode(request.path, true),
        canonical_query(&request.query),
    );
    let to_sign = format!(
        "{ALGORITHM}\n{}\n{}\n{}",
        amz_date(now),
        scope(key, now),
        sha256_hex(canonical.as_bytes())
    );
    let date = now.format("%Y%m%d").to_string();
    let signing = signing_key(key.secret, &date, key.region, key.service);
    hex(&hmac(&*signing, &to_sign))
}

/// The query string of a presigned URL for `request`, valid `expires` seconds from `now`: the
/// request's own query, the signing fields and the signature. The headers in `request` must be
/// sent as given.
pub(super) fn presign(
    key: &Key<'_>,
    mut request: Request<'_>,
    now: DateTime<Utc>,
    expires: u32,
) -> String {
    let (_, signed) = canonical_headers(&request);
    let credential = format!("{}/{}", key.id, scope(key, now));
    request.query.extend([
        ("X-Amz-Algorithm".to_owned(), ALGORITHM.to_owned()),
        ("X-Amz-Credential".to_owned(), credential),
        ("X-Amz-Date".to_owned(), amz_date(now)),
        ("X-Amz-Expires".to_owned(), expires.to_string()),
        ("X-Amz-SignedHeaders".to_owned(), signed),
    ]);
    let signature = signature(key, &request, UNSIGNED_PAYLOAD, now);
    format!(
        "{}&X-Amz-Signature={signature}",
        canonical_query(&request.query)
    )
}

/// The headers that sign `request` whose body hashes to `payload` (hex SHA-256): its own
/// headers, then `x-amz-content-sha256`, `x-amz-date` and `authorization`.
pub(super) fn signed_headers(
    key: &Key<'_>,
    mut request: Request<'_>,
    payload: &str,
    now: DateTime<Utc>,
) -> Vec<(String, String)> {
    request.headers.extend([
        ("x-amz-content-sha256".to_owned(), payload.to_owned()),
        ("x-amz-date".to_owned(), amz_date(now)),
    ]);
    let (_, signed) = canonical_headers(&request);
    let signature = signature(key, &request, payload, now);
    let authorization = format!(
        "{ALGORITHM} Credential={}/{},SignedHeaders={signed},Signature={signature}",
        key.id,
        scope(key, now)
    );
    let mut headers = request.headers;
    headers.push(("authorization".to_owned(), authorization));
    headers
}
