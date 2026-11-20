//! Foreground Android reporting through the active link and the hub's device-scoped daemon API.

mod readings;

use std::time::Duration;

use daemon_core::wire::{EnrolRequest, Enrolled, EnrolmentCode};
use reqwest::{Client, RequestBuilder};
use tauri::{Manager, Webview};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::tokens;

const SCOPE: &str = "ing.anima.mobile.metrics";

fn post(webview: &Webview, account: &str, origin: &str, action: &str) -> Option<RequestBuilder> {
    let socket = webview
        .try_state::<crate::phone_link::PhoneLink>()?
        .metrics_socket(origin, account)?;
    let client = Client::builder()
        .unix_socket(socket)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    Some(client.post(format!("http://localhost/daemon/v1/{action}")))
}

async fn credential(webview: &Webview, account: &str) -> Option<Enrolled> {
    let value = Zeroizing::new(tokens::store(webview).read(SCOPE, account).await.ok()??);
    serde_json::from_str(&value).ok()
}

#[tauri::command]
pub async fn mobile_metrics_enrolled(webview: Webview, origin: String, device: Uuid) -> bool {
    let Some(account) = tokens::reporting_link(&webview, &origin).await else {
        return false;
    };
    credential(&webview, &account)
        .await
        .is_some_and(|stored| stored.device == device && stored.expires_at > chrono::Utc::now())
}

#[tauri::command]
pub async fn mobile_metrics_enrol(
    webview: Webview,
    origin: String,
    device: Uuid,
    code: String,
) -> Result<(), &'static str> {
    let code = Zeroizing::new(code);
    let code = EnrolmentCode::parse(&code).ok_or("invalid enrolment code")?;
    let account = tokens::reporting_link(&webview, &origin)
        .await
        .ok_or("no signed-in device link")?;
    let enrolled: Enrolled = post(&webview, &account, &origin, "enrol")
        .ok_or("device link changed")?
        .json(&EnrolRequest { code })
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| "hub refused metrics enrolment")?
        .json()
        .await
        .map_err(|_| "hub did not return a reporting credential")?;
    if enrolled.device != device {
        return Err("hub enrolled a different device");
    }
    let stored = Zeroizing::new(
        serde_json::to_string(&enrolled).map_err(|_| "could not encode reporting credential")?,
    );
    tokens::store(&webview)
        .write(SCOPE, &account, &stored)
        .await
        .map_err(|_| "could not keep reporting credential")
}

#[tauri::command]
pub async fn mobile_metrics_push(webview: Webview, origin: String) -> Result<(), &'static str> {
    send(&webview, &origin)
        .await
        .ok_or("could not report tablet readings")
}

async fn send(webview: &Webview, origin: &str) -> Option<()> {
    let account = tokens::reporting_link(webview, origin).await?;
    let enrolled = credential(webview, &account).await?;
    let now = chrono::Utc::now();
    let path = webview.path().app_local_data_dir().ok()?;
    let sample = readings::batch(enrolled.device, now, &path)?;
    post(webview, &account, origin, "push")?
        .bearer_auth(enrolled.token.expose())
        .json(&sample)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    Some(())
}
