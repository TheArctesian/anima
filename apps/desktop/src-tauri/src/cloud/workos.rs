//! The device authorization flow of `WorkOS` `AuthKit` (RFC 8628) as a public client: the client ID
//! only, no API key and no secret (HMD-032). The endpoints and fields are those of
//! <https://workos.com/docs/authkit/cli-auth> and its reference pages, read on 2026-11-17.

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use zeroize::Zeroizing;

use super::CloudError;

const DEVICE_URL: &str = "https://api.workos.com/user_management/authorize/device";
const TOKEN_URL: &str = "https://api.workos.com/user_management/authenticate";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// One started sign-in. The device code stays in the shell: the guide says it is never shown.
pub(super) struct DeviceCode {
    pub secret: Zeroizing<String>,
    pub user_code: String,
    pub page: String,
    pub interval: u64,
}

/// The tokens of a sign-in or a refresh. No `Debug`: they are secrets.
pub(super) struct Tokens {
    pub access: Zeroizing<String>,
    pub refresh: Zeroizing<String>,
}

/// One poll of a sign-in.
pub(super) enum Poll {
    Waiting,
    /// `slow_down`: wait at least 5 s more before the next poll.
    SlowDown,
    SignedIn(Tokens),
}

#[derive(Deserialize)]
struct DeviceAnswer {
    device_code: String,
    user_code: String,
    verification_uri_complete: String,
    #[serde(default = "five")]
    interval: u64,
}

const fn five() -> u64 {
    5
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct ErrorAnswer {
    error: String,
}

/// Starts a sign-in.
pub(super) async fn start(http: &Client, client_id: &str) -> Result<DeviceCode, CloudError> {
    let response = http
        .post(DEVICE_URL)
        .form(&[("client_id", client_id)])
        .send()
        .await?;
    let (status, body) = (response.status(), response.bytes().await?);
    device_code(status, &body)
}

/// Asks once whether the person signed in with `device_code`.
pub(super) async fn poll(
    http: &Client,
    client_id: &str,
    device_code: &str,
) -> Result<Poll, CloudError> {
    let form = [
        ("grant_type", DEVICE_GRANT),
        ("device_code", device_code),
        ("client_id", client_id),
    ];
    let response = http.post(TOKEN_URL).form(&form).send().await?;
    let (status, body) = (
        response.status(),
        Zeroizing::new(response.bytes().await?.to_vec()),
    );
    polled(status, &body)
}

/// New tokens for the refresh token. A refused refresh token is `Unauthenticated`.
pub(super) async fn refresh(
    http: &Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<Tokens, CloudError> {
    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];
    let response = http.post(TOKEN_URL).form(&form).send().await?;
    let (status, body) = (
        response.status(),
        Zeroizing::new(response.bytes().await?.to_vec()),
    );
    match polled(status, &body)? {
        Poll::SignedIn(tokens) => Ok(tokens),
        Poll::Waiting | Poll::SlowDown => Err(CloudError::Refused),
    }
}

/// The answer of the device authorization request.
pub(super) fn device_code(status: StatusCode, body: &[u8]) -> Result<DeviceCode, CloudError> {
    if !status.is_success() {
        return Err(refusal(status, body));
    }
    let answer: DeviceAnswer = serde_json::from_slice(body).map_err(|_| CloudError::Refused)?;
    Ok(DeviceCode {
        secret: Zeroizing::new(answer.device_code),
        user_code: answer.user_code,
        page: answer.verification_uri_complete,
        interval: answer.interval.max(1),
    })
}

/// The answer of the token request: tokens, a wait, or why the sign-in stopped.
pub(super) fn polled(status: StatusCode, body: &[u8]) -> Result<Poll, CloudError> {
    if status.is_success() {
        let answer: TokenAnswer = serde_json::from_slice(body).map_err(|_| CloudError::Refused)?;
        return Ok(Poll::SignedIn(Tokens {
            access: Zeroizing::new(answer.access_token),
            refresh: Zeroizing::new(answer.refresh_token),
        }));
    }
    match error_code(body).as_deref() {
        Some("authorization_pending") => Ok(Poll::Waiting),
        Some("slow_down") => Ok(Poll::SlowDown),
        _ => Err(refusal(status, body)),
    }
}

/// The refusal that a failed answer means (the error codes of the device-code reference).
fn refusal(status: StatusCode, body: &[u8]) -> CloudError {
    if status.is_server_error() {
        return CloudError::Offline;
    }
    match error_code(body).as_deref() {
        Some("access_denied") => CloudError::Cancelled,
        Some("expired_token") => CloudError::Expired,
        Some("invalid_grant") => CloudError::Unauthenticated,
        _ => CloudError::Refused,
    }
}

fn error_code(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<ErrorAnswer>(body)
        .ok()
        .map(|answer| answer.error)
}
