//! "Stay awake" (AWK-001 to AWK-008): keeps this computer from sleeping so a hub or an agent
//! on it keeps working. Three levels: off, no idle sleep, and on with the lid closed. The
//! Settings page and the tray item change it; both show each change.
//!
//! Only the main web view may ask (WEB-007). Every hold ends with the app (AWK-007): the OS
//! ties each hold program to the app's process, and `release` runs at exit.

mod control;
mod error;
mod hold;
mod level;
mod lid_set_up;
mod platform;
mod power;
mod saved;
mod tray;

#[cfg(test)]
mod tests;

use std::{
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    thread,
    time::Duration,
};

use tauri::{AppHandle, Emitter, EventTarget, Manager, State, Webview};
use tauri_plugin_log::log;

use control::{Control, ControlError, Status};
pub use error::StayAwakeError;
use level::Level;
use platform::Platform;

use crate::web_view::MAIN;

/// The event the main web view gets with each new state.
const EVENT: &str = "stay-awake";

/// How often the battery and the hold programs get a look.
const WATCH: Duration = Duration::from_secs(60);

/// The control of this computer, shared by the commands, the tray and the watch.
#[derive(Clone)]
pub struct StayAwake(Arc<Mutex<Control>>);

impl StayAwake {
    fn control(&self) -> MutexGuard<'_, Control> {
        // A panic in one change must not end the control: the state inside is still whole.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Ends every hold. The app calls it at exit (AWK-007).
    pub fn release(&self) {
        self.control().release();
    }
}

/// Starts the control with the saved choice, makes the tray item and starts the watch.
pub fn start(app: &AppHandle) {
    let Ok(folder) = app.path().app_config_dir() else {
        log::warn!("stay awake is off: the app has no configuration folder");
        return;
    };
    let shared = StayAwake(Arc::new(Mutex::new(Control::open(
        Platform::here(),
        folder,
    ))));
    let status = shared.control().refresh();
    app.manage(shared.clone());
    if tray::build(app, status).is_err() {
        log::warn!("the tray item could not be made");
    }
    let app = app.clone();
    thread::spawn(move || watch(&app, &shared, status));
}

/// Looks at the battery and the hold programs once a minute while a level is chosen (AWK-006).
fn watch(app: &AppHandle, shared: &StayAwake, mut shown: Status) {
    loop {
        thread::sleep(WATCH);
        let status = {
            let mut control = shared.control();
            if !control.active() {
                continue;
            }
            control.refresh()
        };
        if status != shown {
            publish(app, status);
        }
        shown = status;
    }
}

/// Shows `status` in the tray and sends it to the main web view.
fn publish(app: &AppHandle, status: Status) {
    tray::show(app, status);
    if app
        .emit_to(EventTarget::webview_window(MAIN), EVENT, status)
        .is_err()
    {
        log::warn!("a stay-awake state did not reach the window");
    }
}

/// Runs one change of the control off the async threads, then shows the new state.
async fn change(
    app: AppHandle,
    job: impl FnOnce(&mut Control) -> Result<Status, ControlError> + Send + 'static,
) -> Result<Status, StayAwakeError> {
    let Some(shared) = app
        .try_state::<StayAwake>()
        .map(|state| state.inner().clone())
    else {
        return Err(StayAwakeError::Failed);
    };
    let done = tauri::async_runtime::spawn_blocking(move || job(&mut shared.control()))
        .await
        .map_err(|_| StayAwakeError::Failed)?;
    match done {
        Ok(status) => {
            publish(&app, status);
            Ok(status)
        }
        Err(error) => {
            let error = StayAwakeError::from(error);
            log::warn!("a stay-awake change failed: {}", error.code());
            Err(error)
        }
    }
}

/// The tray chose `level`. The tray has no place for an error: the lines show the state that is.
fn choose_from_tray(app: &AppHandle, level: Level) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if change(app.clone(), move |control| control.choose(level))
            .await
            .is_err()
            && let Some(shared) = app.try_state::<StayAwake>()
        {
            let status = shared.control().refresh();
            tray::show(&app, status);
        }
    });
}

fn from_main(label: &str) -> Result<(), StayAwakeError> {
    if label == MAIN {
        Ok(())
    } else {
        Err(StayAwakeError::Caller)
    }
}

/// The state of the control now.
#[tauri::command]
pub async fn stay_awake_read(
    webview: Webview,
    state: State<'_, StayAwake>,
) -> Result<Status, StayAwakeError> {
    from_main(webview.label())?;
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || shared.control().refresh())
        .await
        .map_err(|_| StayAwakeError::Failed)
}

/// Chooses a level (AWK-001).
#[tauri::command]
pub async fn stay_awake_choose(
    webview: Webview,
    app: AppHandle,
    level: Level,
) -> Result<Status, StayAwakeError> {
    from_main(webview.label())?;
    change(app, move |control| control.choose(level)).await
}

/// Sets the battery floor in percent, or removes it (AWK-006).
#[tauri::command]
pub async fn stay_awake_floor(
    webview: Webview,
    app: AppHandle,
    floor: Option<u8>,
) -> Result<Status, StayAwakeError> {
    from_main(webview.label())?;
    change(app, move |control| control.set_floor(floor)).await
}

/// Runs the set-up step of the lid-closed level; macOS asks for an administrator (AWK-003).
#[tauri::command]
pub async fn stay_awake_lid_set_up(
    webview: Webview,
    app: AppHandle,
) -> Result<Status, StayAwakeError> {
    from_main(webview.label())?;
    change(app, Control::set_up_lid).await
}

/// Removes the set-up step of the lid-closed level; macOS asks for an administrator.
#[tauri::command]
pub async fn stay_awake_lid_remove(
    webview: Webview,
    app: AppHandle,
) -> Result<Status, StayAwakeError> {
    from_main(webview.label())?;
    change(app, Control::remove_lid_set_up).await
}
