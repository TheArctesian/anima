//! Device web views: a child of the hidden window with its own store and the http(s)-only
//! navigation filter (D-2, D-3).

use tauri::{AppHandle, LogicalPosition, LogicalSize, Url, Webview, WebviewBuilder, WebviewUrl};

use super::Harness;
use crate::event::Event;

/// The store identifiers the harness uses (16 bytes, as a `DeviceId` UUID would be).
pub const STORES: &[[u8; 16]] = &[STORE_A, STORE_B, STORE_IPC, STORE_TLS, STORE_NAV];
pub const STORE_A: [u8; 16] = *b"anima-od7-dev-a\0";
pub const STORE_B: [u8; 16] = *b"anima-od7-dev-b\0";
pub const STORE_IPC: [u8; 16] = *b"anima-od7-ipc--\0";
pub const STORE_TLS: [u8; 16] = *b"anima-od7-tls--\0";
pub const STORE_NAV: [u8; 16] = *b"anima-od7-nav--\0";

pub fn is_web_scheme(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

/// Opens a device web view. `store: None` means the default store (the main view's).
pub fn open(harness: &Harness, label: &str, store: Option<[u8; 16]>, url: Url) -> Result<Webview, String> {
    let events = harness.events.clone();
    let owner = label.to_owned();
    let mut builder = WebviewBuilder::new(label, WebviewUrl::External(url))
        .focused(false)
        .on_navigation(move |url| {
            let allowed = is_web_scheme(url);
            let mut shown = url.to_string();
            shown.truncate(60);
            events.send(Event::Navigation { label: owner.clone(), url: shown, allowed });
            allowed
        });
    if let Some(store) = store {
        builder = with_store(builder, store);
    }
    harness
        .window
        .add_child(builder, LogicalPosition::new(0.0, 0.0), LogicalSize::new(400.0, 300.0))
        .map_err(|e| format!("{label}: {e}"))
}

#[cfg(target_os = "macos")]
fn with_store(builder: WebviewBuilder<tauri::Wry>, store: [u8; 16]) -> WebviewBuilder<tauri::Wry> {
    builder.data_store_identifier(store)
}

#[cfg(not(target_os = "macos"))]
fn with_store(builder: WebviewBuilder<tauri::Wry>, store: [u8; 16]) -> WebviewBuilder<tauri::Wry> {
    builder.data_directory(store_directory(store))
}

#[cfg(not(target_os = "macos"))]
fn store_directory(store: [u8; 16]) -> std::path::PathBuf {
    let name: String = store.iter().map(|b| format!("{b:02x}")).collect();
    std::env::temp_dir().join("anima-od7-spike").join(name)
}

/// Removes the stores of earlier runs so every check starts empty.
#[cfg(target_os = "macos")]
pub async fn clear_stores(app: &AppHandle) {
    for store in STORES {
        if let Err(e) = app.remove_data_store(*store).await {
            eprintln!("od7: no earlier store to remove ({e})");
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub async fn clear_stores(_app: &AppHandle) {
    for store in STORES {
        let directory = store_directory(*store);
        if let Err(e) = std::fs::remove_dir_all(&directory) {
            eprintln!("od7: no earlier store to remove ({})", e.kind());
        }
    }
}
