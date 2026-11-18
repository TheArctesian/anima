//! The sign-in to the Anima.ing account and the calls of its cloud (hosted-mode.md Section 4.1,
//! HMD-032 to HMD-035, HMD-043). They run in the shell, not in the web view: the `WorkOS` refresh
//! token stays in the key store and never reaches script (HMD-034), and no browser rule (CORS)
//! of `api.workos.com` or `cloud.anima.ing` decides whether a call may go. No token, device code
//! or answer is logged (LOG-007).

mod api;
mod device;
mod error;
mod name;
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

use crate::this_machine::OperatingSystem;

pub use api::{HubAccess, HubState};
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

fn from_main(webview: &Webview) -> Result<(), CloudError> {
    crate::web_view::caller(webview.label()).map_err(|_| CloudError::Caller)
}

impl Cloud {
    fn set_access(&self, token: Option<Zeroizing<String>>) {
        *self.access.lock().unwrap_or_else(PoisonError::into_inner) = token;
    }

    fn access(&self) -> Option<Zeroizing<String>> {
        self.access
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Keeps the tokens of a sign-in or a refresh: the access token in memory, the refresh
    /// token in the key store in place of the old one (HMD-034).
    fn keep(&self, id: &str, tokens: &workos::Tokens) -> Result<(), CloudError> {
        session::write(id, &tokens.refresh)?;
        self.set_access(Some(tokens.access.clone()));
        Ok(())
    }

    /// A new access token from the stored refresh token. A refused refresh token is deleted.
    async fn refreshed(&self, id: &str) -> Result<(), CloudError> {
        let Some(stored) = session::read(id)? else {
            return Err(CloudError::Unauthenticated);
        };
        match workos::refresh(&self.http, id, &stored).await {
            Ok(tokens) => self.keep(id, &tokens),
            Err(CloudError::Unauthenticated) => {
                session::clear(id)?;
                self.set_access(None);
                Err(CloudError::Unauthenticated)
            }
            Err(other) => Err(other),
        }
    }

    /// Runs `call` with the access token; on `Unauthenticated` refreshes once and runs it again.
    async fn with_access<T, F, R>(&self, call: F) -> Result<T, CloudError>
    where
        F: Fn(Zeroizing<String>) -> R,
        R: std::future::Future<Output = Result<T, CloudError>>,
    {
        let id = client_id()?;
        if self.access().is_none() {
            self.refreshed(id).await?;
        }
        let token = self.access().ok_or(CloudError::Unauthenticated)?;
        match call(token).await {
            Err(CloudError::Unauthenticated) => {
                self.refreshed(id).await?;
                let token = self.access().ok_or(CloudError::Unauthenticated)?;
                call(token).await
            }
            other => other,
        }
    }
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
            cloud.keep(id, &tokens)?;
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
    match cloud.refreshed(client_id()?).await {
        Ok(()) => Ok(true),
        Err(CloudError::Unauthenticated) => Ok(false),
        Err(other) => Err(other),
    }
}

/// `POST /v1/hub`: makes or wakes the hub of the account and says where it is.
#[tauri::command]
pub async fn cloud_hub(webview: Webview, cloud: State<'_, Cloud>) -> Result<HubState, CloudError> {
    from_main(&webview)?;
    let http = cloud.http.clone();
    cloud
        .with_access(|token| {
            let http = http.clone();
            async move { api::hub(&http, CLOUD_URL, &token).await }
        })
        .await
}

/// `POST /v1/hub/access` for this computer, named from its computer name, which stays the same
/// across sign-ins, so a second sign-in finds the device on the link. While the hub is not
/// ready, it asks `/v1/hub` again (which makes or wakes it) every 2 s, for up to 90 s.
#[tauri::command]
pub async fn cloud_hub_access(
    webview: Webview,
    cloud: State<'_, Cloud>,
) -> Result<HubAccess, CloudError> {
    from_main(&webview)?;
    let device = name::device_name(&crate::this_machine::computer_name());
    let http = cloud.http.clone();
    let started = tokio::time::Instant::now();
    loop {
        let answered = cloud
            .with_access(|token| {
                let (http, device) = (http.clone(), device.clone());
                async move { api::hub_access(&http, CLOUD_URL, &token, &device).await }
            })
            .await;
        if !matches!(answered, Err(CloudError::NotReady)) || started.elapsed() >= NOT_READY_LIMIT {
            return answered;
        }
        tokio::time::sleep(NOT_READY_WAIT).await;
        let _ = cloud
            .with_access(|token| {
                let http = http.clone();
                async move { api::hub(&http, CLOUD_URL, &token).await }
            })
            .await;
    }
}

/// This computer as the hosted hub lists it (HMD-044): the device name that `cloud_hub_access`
/// sends, the computer name and the OS. It calls no network and reads no secret.
#[tauri::command]
pub async fn cloud_device(webview: Webview) -> Result<CloudDevice, CloudError> {
    from_main(&webview)?;
    // The sign-in is only on macOS and Linux (`capabilities/cloud.json`).
    let os = OperatingSystem::current().ok_or(CloudError::Unconfigured)?;
    Ok(device::cloud_device(
        &crate::this_machine::computer_name(),
        os,
    ))
}
