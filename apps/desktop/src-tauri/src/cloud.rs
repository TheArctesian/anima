//! The sign-in to the Anima.ing account and the calls of its cloud (hosted-mode.md Section 4.1,
//! HMD-032 to HMD-035, HMD-043). They run in the shell, not in the web view: the `WorkOS` refresh
//! token stays in the key store and never reaches script (HMD-034), and no browser rule (CORS)
//! of `api.workos.com` or `cloud.anima.ing` decides whether a call may go. No token, device code
//! or answer is logged (LOG-007).
//!
//! The desktop application and the phone application have the same commands (mobile plan M3-4). They
//! differ in three places only: where the refresh token is kept, how the device is named, and
//! `cloud_device`, which only a computer has (HMD-044).

mod api;
#[cfg(desktop)]
mod device;
mod error;
#[cfg(mobile)]
pub mod in_app;
mod lifecycle;
#[cfg(any(target_os = "ios", test))]
mod loopback;
mod name;
#[cfg(any(mobile, test))]
mod phone;
#[cfg(any(target_os = "ios", test))]
mod pkce;
#[cfg(desktop)]
mod session;
#[cfg(test)]
mod tests;
mod workos;

use std::{
    sync::{Mutex, PoisonError},
    time::Duration,
};

use serde::Serialize;
use tauri::{State, Webview};
use zeroize::Zeroizing;

#[cfg(mobile)]
use crate::mobile::account_session as session;
#[cfg(desktop)]
use crate::this_machine::OperatingSystem;

pub use api::{HubAccess, HubState};
#[cfg(desktop)]
pub use device::CloudDevice;
pub use error::CloudError;

/// The public client ID of the app in `WorkOS`: the production client, which the site checks
/// tokens against. A build can set another one (a staging client) with `ANIMA_CLOUD_CLIENT_ID`;
/// set empty, the build has no sign-in. A client ID is public (HMD-035).
const CLIENT_ID: &str = match option_env!("ANIMA_CLOUD_CLIENT_ID") {
    Some(id) => id,
    None => "client_01M7M1VFKREYV6H5714SDVS4E6",
};

/// The cloud, set when the app is built for a self-host person (HCL-002).
const CLOUD_URL: &str = match option_env!("ANIMA_CLOUD_URL") {
    Some(url) => url,
    None => "https://cloud.anima.ing",
};

/// How long one call may take.
const CALL_LIMIT: Duration = Duration::from_secs(20);
/// The extra wait that `slow_down` asks for.
const SLOW_DOWN: Duration = Duration::from_secs(5);
/// While the hub is not ready for an access: the wait between two asks, and how long to ask.
const NOT_READY_WAIT: Duration = Duration::from_secs(2);
const NOT_READY_LIMIT: Duration = Duration::from_secs(90);

/// The sign-in of this app: the device code of a sign-in that runs, and the `WorkOS` access token
/// of the account, in memory only.
pub struct Cloud {
    http: reqwest::Client,
    pending: Mutex<Option<Zeroizing<String>>>,
    access: Mutex<Option<Zeroizing<String>>>,
    session: tokio::sync::Mutex<()>,
}

impl Default for Cloud {
    fn default() -> Self {
        let http = reqwest::Client::builder()
            .timeout(CALL_LIMIT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default();
        Self {
            http,
            pending: Mutex::new(None),
            access: Mutex::new(None),
            session: tokio::sync::Mutex::new(()),
        }
    }
}

/// What the client shows of a started sign-in. The device code is not in it.
#[derive(Serialize)]
pub struct SignInCode {
    code: String,
    page: String,
    interval: u64,
}

fn client_id() -> Result<&'static str, CloudError> {
    if CLIENT_ID.is_empty() {
        Err(CloudError::Unconfigured)
    } else {
        Ok(CLIENT_ID)
    }
}

#[cfg(desktop)]
fn from_main(webview: &Webview) -> Result<(), CloudError> {
    crate::web_view::caller(webview.label()).map_err(|_| CloudError::Caller)
}

/// A phone has one web view; the capability names it, and this is the second wall.
#[cfg(mobile)]
fn from_main(webview: &Webview) -> Result<(), CloudError> {
    (webview.label() == "main")
        .then_some(())
        .ok_or(CloudError::Caller)
}

/// The name of this computer on the link of the hub: from its computer name, which stays the same
/// across sign-ins, so a second sign-in finds the device on the link.
#[cfg(desktop)]
fn device_name(_: &Webview) -> String {
    name::device_name(&crate::this_machine::computer_name())
}

/// The name of this phone on the link of the hub: kept in the data folder of the app, so a second
/// sign-in finds the device on the link.
#[cfg(mobile)]
fn device_name(webview: &Webview) -> String {
    use tauri::Manager;
    let kind = if cfg!(target_os = "ios") {
        "iphone"
    } else {
        "android"
    };
    let folder = webview.path().app_local_data_dir().ok();
    phone::device_name(folder.as_deref(), kind, || {
        let mut random = [0_u8; 3];
        getrandom::fill(&mut random).ok().map(|()| random)
    })
}

/// Starts a sign-in: the short code to show and the page to open in the system browser.
#[tauri::command]
pub async fn cloud_sign_in_start(
    webview: Webview,
    cloud: State<'_, Cloud>,
) -> Result<SignInCode, CloudError> {
    from_main(&webview)?;
    let started = workos::start(&cloud.http, client_id()?).await?;
    *cloud.pending.lock().unwrap_or_else(PoisonError::into_inner) = Some(started.secret);
    Ok(SignInCode {
        code: started.user_code,
        page: started.page,
        interval: started.interval,
    })
}

/// Asks once whether the person signed in: `false` while the sign-in waits.
#[tauri::command]
pub async fn cloud_sign_in_poll(
    webview: Webview,
    cloud: State<'_, Cloud>,
) -> Result<bool, CloudError> {
    from_main(&webview)?;
    let id = client_id()?;
    let pending = cloud
        .pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let device_code = pending.ok_or(CloudError::Expired)?;
    match workos::poll(&cloud.http, id, &device_code).await? {
        workos::Poll::Waiting => Ok(false),
        workos::Poll::SlowDown => {
            tokio::time::sleep(SLOW_DOWN).await;
            Ok(false)
        }
        workos::Poll::SignedIn(tokens) => {
            *cloud.pending.lock().unwrap_or_else(PoisonError::into_inner) = None;
            cloud.keep(&webview, id, &tokens).await?;
            Ok(true)
        }
    }
}

/// Signs in again with no step of the person when the key store holds a refresh token that
/// `WorkOS` still takes: `true` when it did.
#[tauri::command]
pub async fn cloud_sign_in_resume(
    webview: Webview,
    cloud: State<'_, Cloud>,
) -> Result<bool, CloudError> {
    from_main(&webview)?;
    match cloud.refreshed(&webview, client_id()?).await {
        Ok(()) => Ok(true),
        Err(CloudError::Unauthenticated) => Ok(false),
        Err(other) => Err(other),
    }
}

/// Drops this phone's local Anima.ing account session.
#[cfg(mobile)]
#[tauri::command]
pub async fn cloud_sign_out(webview: Webview, cloud: State<'_, Cloud>) -> Result<(), CloudError> {
    from_main(&webview)?;
    let _guard = cloud.session.lock().await;
    cloud.set_access(None);
    *cloud.pending.lock().unwrap_or_else(PoisonError::into_inner) = None;
    if let Ok(id) = client_id() {
        session::clear(&webview, id).await?;
    }
    Ok(())
}

/// `POST /v1/hub`: makes or wakes the hub of the account and says where it is.
#[tauri::command]
pub async fn cloud_hub(webview: Webview, cloud: State<'_, Cloud>) -> Result<HubState, CloudError> {
    from_main(&webview)?;
    let http = cloud.http.clone();
    cloud
        .with_access(&webview, |token| {
            let http = http.clone();
            async move { api::hub(&http, CLOUD_URL, &token).await }
        })
        .await
}

/// `POST /v1/hub/access` for this device, with its name on the link (`device_name`). While the hub
/// is not ready, it asks `/v1/hub` again (which makes or wakes it) every 2 s, for up to 90 s.
#[tauri::command]
pub async fn cloud_hub_access(
    webview: Webview,
    cloud: State<'_, Cloud>,
) -> Result<HubAccess, CloudError> {
    from_main(&webview)?;
    let device = device_name(&webview);
    let http = cloud.http.clone();
    let started = tokio::time::Instant::now();
    loop {
        let answered = cloud
            .with_access(&webview, |token| {
                let (http, device) = (http.clone(), device.clone());
                async move { api::hub_access(&http, CLOUD_URL, &token, &device).await }
            })
            .await;
        if !matches!(answered, Err(CloudError::NotReady)) || started.elapsed() >= NOT_READY_LIMIT {
            #[cfg(desktop)]
            if let Ok(access) = &answered {
                crate::link::hosted_hub_opened(&webview, access.is_joined());
            }
            return answered;
        }
        tokio::time::sleep(NOT_READY_WAIT).await;
        let _ = cloud
            .with_access(&webview, |token| {
                let http = http.clone();
                async move { api::hub(&http, CLOUD_URL, &token).await }
            })
            .await;
    }
}

/// This computer as the hosted hub lists it (HMD-044): the device name that `cloud_hub_access`
/// sends, the computer name and the OS. It calls no network and reads no secret. A phone has no
/// such command: the client adds only a computer to the devices of the hub.
#[cfg(desktop)]
#[tauri::command]
pub async fn cloud_device(webview: Webview) -> Result<CloudDevice, CloudError> {
    from_main(&webview)?;
    // The sign-in is only on macOS and Linux (`capabilities/cloud.json`).
    let os = OperatingSystem::current().ok_or(CloudError::Unconfigured)?;
    // The client asks this only for a hosted hub, at each start: the shell then knows that a
    // unit of the user must keep the link, also when no sign-in runs (HMD-058).
    crate::link::hosted_hub_opened(&webview, true);
    Ok(device::cloud_device(
        &crate::this_machine::computer_name(),
        os,
    ))
}
