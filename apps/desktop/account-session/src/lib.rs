//! Shows the sign-in page of the Anima.ing account inside the iPhone app (mobile plan M3-5).
//!
//! iOS has a web authentication session for this: a page over the app that shares the sign-in of Safari. The
//! plugin shows a page and takes it away; it knows nothing of the sign-in. The shell gets the result at a loopback
//! address. The web view has no command here. On Android and on a computer the plugin does nothing.

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_account_session);

/// The page of one sign-in, on iOS.
#[cfg(target_os = "ios")]
pub struct AccountSession<R: Runtime>(tauri::plugin::PluginHandle<R>);

/// Why the page did not show.
#[cfg(target_os = "ios")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotShown;

#[cfg(target_os = "ios")]
#[derive(serde::Serialize)]
struct Open<'a> {
    url: &'a str,
}

#[cfg(target_os = "ios")]
impl<R: Runtime> AccountSession<R> {
    /// Shows the page at `url` (`https` only) and returns when it is gone: the person closed it, or `close` took
    /// it away. It blocks, so call it off the main thread.
    ///
    /// # Errors
    ///
    /// `NotShown` when iOS did not show the page or a page already shows.
    pub fn open(&self, url: &str) -> Result<(), NotShown> {
        self.0
            .run_mobile_plugin::<()>("open", Open { url })
            .map_err(|_| NotShown)
    }

    /// Takes the page away. With no page it does nothing.
    pub fn close(&self) {
        let _ = self.0.run_mobile_plugin::<()>("close", ());
    }
}

/// The plugin. Register it once, in the mobile application.
#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("account-session")
        .setup(|app, api| {
            #[cfg(target_os = "ios")]
            {
                use tauri::Manager;
                let handle = api.register_ios_plugin(init_plugin_account_session)?;
                app.manage(AccountSession(handle));
            }
            #[cfg(not(target_os = "ios"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
