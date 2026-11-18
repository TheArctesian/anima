//! Keeps the Android web view above the on-screen keyboard (MOB-011, mobile plan M4-1).
//!
//! The app draws edge to edge, so Android does not make the web view shorter when the keyboard opens, and the web view
//! does not tell the page that the keyboard is there. `KeyboardPlugin.kt` makes the web view end where the keyboard
//! starts. On iOS the web view reports the keyboard itself, and the plugin does nothing.

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// The plugin. Register it once, in the mobile application.
#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("keyboard")
        .setup(|_app, api| {
            #[cfg(target_os = "android")]
            api.register_android_plugin("ing.anima.keyboard", "KeyboardPlugin")?;
            #[cfg(not(target_os = "android"))]
            let _ = api;
            Ok(())
        })
        .build()
}
