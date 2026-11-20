//! The two calls of the Anima.ing cloud (HMD-043): the hub of the account, and the access of this
//! device to it. Each sends the `WorkOS` access token as a bearer. The routes and their answers
//! are those of the site's hub plan (`/v1/hub`, `/v1/hub/access`, 2026-11-17).

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::CloudError;

/// The hub of the account: ready, or a job of the cloud that makes or wakes it (HMD-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HubState {
    Ready,
    Making,
    Waking,
}

#[derive(Deserialize)]
struct HubAnswer {
    state: HubState,
}

/// What the cloud gives this device for the ready hub: the link invite (none when this device
/// is already on the hub's link) and the one-time claim that the device trades with the hub,
/// through the link, for its own token (R-129, HMD-040). Both go to the main web view, which
/// joins with them at once. No `Debug`: they are secrets.
#[derive(Deserialize, Serialize)]
pub struct HubAccess {
    invite: Option<String>,
    claim: String,
}

#[cfg(desktop)]
impl HubAccess {
    /// Whether this device is on the link of the hub already: the cloud then gives no invite.
    pub fn is_joined(&self) -> bool {
        self.invite.is_none()
    }
}

impl Drop for HubAccess {
    fn drop(&mut self) {
        if let Some(invite) = self.invite.as_mut() {
            zeroize::Zeroize::zeroize(invite);
        }
        zeroize::Zeroize::zeroize(&mut self.claim);
    }
}

#[derive(Deserialize)]
struct ErrorAnswer {
    error: ErrorCode,
}

#[derive(Deserialize)]
struct ErrorCode {
    code: String,
}

/// `POST /v1/hub`: makes or wakes the hub of the account and says where it is.
pub(super) async fn hub(http: &Client, base: &str, access: &str) -> Result<HubState, CloudError> {
    let response = http
        .post(format!("{base}/v1/hub"))
        .bearer_auth(access)
        // The site refuses a POST that is not JSON (SvelteKit's cross-site form check).
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?;
    let (status, body) = (response.status(), response.bytes().await?);
    if !status.is_success() {
        return Err(refusal(status, &body));
    }
    let answer: HubAnswer = serde_json::from_slice(&body).map_err(|_| CloudError::Refused)?;
    Ok(answer.state)
}

/// `POST /v1/hub/access` with `{"device": name}`.
pub(super) async fn hub_access(
    http: &Client,
    base: &str,
    access: &str,
    device: &str,
) -> Result<HubAccess, CloudError> {
    let body = serde_json::json!({ "device": device }).to_string();
    let response = http
        .post(format!("{base}/v1/hub/access"))
        .bearer_auth(access)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await?;
    let (status, body) = (
        response.status(),
        Zeroizing::new(response.bytes().await?.to_vec()),
    );
    if !status.is_success() {
        return Err(refusal(status, &body));
    }
    serde_json::from_slice(&body).map_err(|_| CloudError::Refused)
}

/// The site's errors: 401 signs in again, 402 `payment_failed` asks for the payment, 409
/// `not_ready` asks `/v1/hub` again, 5xx (`cloud_unavailable`, `not_configured`) is no answer,
/// and any other 4xx is a refusal.
pub(super) fn refusal(status: StatusCode, body: &[u8]) -> CloudError {
    if status == StatusCode::UNAUTHORIZED {
        return CloudError::Unauthenticated;
    }
    if status.is_server_error() {
        return CloudError::Offline;
    }
    let code = serde_json::from_slice::<ErrorAnswer>(body).map(|answer| answer.error.code);
    match (status, code.as_deref()) {
        (StatusCode::PAYMENT_REQUIRED, _) => CloudError::PaymentFailed,
        (StatusCode::CONFLICT, Ok("not_ready")) => CloudError::NotReady,
        _ => CloudError::Refused,
    }
}
