//! The fake R2's own check of Signature Version 4, written apart from the hub's, from the
//! request as it came over the wire: a presigned URL must be in date and at most 15 minutes
//! long (D-28), and a header-signed request must be dated within 5 minutes of the fake's clock,
//! as R2 refuses a skewed one.

use std::fmt::Write as _;

use axum::http::{HeaderMap, Uri, request::Parts};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::{Digest as _, Sha256};

use super::fake_r2::{KEY_ID, SECRET};

/// The longest presigned URL the fake takes, in seconds (D-28).
const MAX_EXPIRES: i64 = 900;

/// How far a header-signed request's date may be from the fake's clock.
const MAX_SKEW: TimeDelta = TimeDelta::minutes(5);

pub fn text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// The raw query pairs, still encoded as sent.
fn query_pairs(uri: &Uri) -> Vec<(String, String)> {
    uri.query()
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (name.to_owned(), value.to_owned())
        })
        .collect()
}

/// What a request's signature claims.
struct Claim {
    date: String,
    credential: String,
    signed_headers: String,
    signature: String,
    payload: String,
    /// The query in the canonical request: every pair but the signature.
    query: Vec<(String, String)>,
}

/// Checks the signature of the request `parts` with `body` at the fake's `now` (`None` takes
/// the request's own date).
pub fn verify(parts: &Parts, body: &[u8], now: Option<DateTime<Utc>>) -> Result<(), String> {
    let (method, uri, headers) = (&parts.method, &parts.uri, &parts.headers);
    let claim = if uri.query().is_some_and(|q| q.contains("X-Amz-Signature=")) {
        presigned(uri, now)?
    } else {
        authorized(headers, body, now)?
    };
    let day = claim.date.get(..8).ok_or("no date")?;
    if claim.credential != format!("{KEY_ID}/{day}/auto/s3/aws4_request") {
        return Err("credential".into());
    }
    let mut query = claim.query.clone();
    query.sort();
    let query: Vec<String> = query.iter().map(|(n, v)| format!("{n}={v}")).collect();
    let mut canonical_headers = String::new();
    for name in claim.signed_headers.split(';') {
        let value = text(headers, name).ok_or_else(|| format!("unsent header {name}"))?;
        let _ = writeln!(canonical_headers, "{name}:{}", value.trim());
    }
    let canonical = format!(
        "{method}\n{}\n{}\n{canonical_headers}\n{}\n{}",
        uri.path(),
        query.join("&"),
        claim.signed_headers,
        claim.payload
    );
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{day}/auto/s3/aws4_request\n{}",
        claim.date,
        hex(&Sha256::digest(canonical.as_bytes()))
    );
    let mut key = format!("AWS4{SECRET}").into_bytes();
    for part in [day, "auto", "s3", "aws4_request"] {
        key = mac(&key, part.as_bytes());
    }
    if hex(&mac(&key, to_sign.as_bytes())) == claim.signature {
        Ok(())
    } else {
        Err("signature".into())
    }
}

fn date(text: &str) -> Result<DateTime<Utc>, String> {
    Ok(NaiveDateTime::parse_from_str(text, "%Y%m%dT%H%M%SZ")
        .map_err(|_| "date")?
        .and_utc())
}

fn presigned(uri: &Uri, now: Option<DateTime<Utc>>) -> Result<Claim, String> {
    let pairs = query_pairs(uri);
    let get = |name: &str| {
        pairs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.replace("%2F", "/").replace("%3B", ";"))
            .ok_or_else(|| format!("no {name}"))
    };
    let signed_at = get("X-Amz-Date")?;
    let expires: i64 = get("X-Amz-Expires")?.parse().map_err(|_| "expires")?;
    if !(1..=MAX_EXPIRES).contains(&expires) {
        return Err("expiry above 15 minutes".into());
    }
    let signed = date(&signed_at)?;
    let now = now.unwrap_or(signed);
    if now < signed || now >= signed + TimeDelta::seconds(expires) {
        return Err("expired".into());
    }
    Ok(Claim {
        date: signed_at,
        credential: get("X-Amz-Credential")?,
        signed_headers: get("X-Amz-SignedHeaders")?,
        signature: get("X-Amz-Signature")?,
        payload: "UNSIGNED-PAYLOAD".into(),
        query: pairs
            .iter()
            .filter(|(n, _)| n != "X-Amz-Signature")
            .cloned()
            .collect(),
    })
}

fn authorized(
    headers: &HeaderMap,
    body: &[u8],
    now: Option<DateTime<Utc>>,
) -> Result<Claim, String> {
    let authorization = text(headers, "authorization").ok_or("unsigned")?;
    let fields = authorization
        .strip_prefix("AWS4-HMAC-SHA256 ")
        .ok_or("algorithm")?;
    let field = |name: &str| {
        fields
            .split(',')
            .find_map(|f| f.trim().strip_prefix(name))
            .map(str::to_owned)
            .ok_or_else(|| format!("no {name}"))
    };
    let payload = text(headers, "x-amz-content-sha256").ok_or("no payload hash")?;
    if payload != hex(&Sha256::digest(body)) {
        return Err("payload hash".into());
    }
    let signed_at = text(headers, "x-amz-date").ok_or("no date")?;
    if let Some(now) = now
        && (date(signed_at)? - now).abs() > MAX_SKEW
    {
        return Err("skewed".into());
    }
    Ok(Claim {
        date: signed_at.to_owned(),
        credential: field("Credential=")?,
        signed_headers: field("SignedHeaders=")?,
        signature: field("Signature=")?,
        payload: payload.to_owned(),
        query: Vec::new(),
    })
}

fn mac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, b| {
        let _ = write!(text, "{b:02x}");
        text
    })
}
