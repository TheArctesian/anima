//! Native inventory for the local runner report (OD-14).

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_runner);

#[cfg(target_os = "ios")]
pub struct Runner<R: Runtime>(tauri::plugin::PluginHandle<R>);

#[cfg(target_os = "ios")]
impl<R: Runtime> Runner<R> {
    /// Reads a fresh native capability report.
    ///
    /// # Errors
    ///
    /// Returns an error when the native plugin could not produce a report.
    pub async fn read(&self) -> Result<serde_json::Value, &'static str> {
        self.0
            .run_mobile_plugin_async("readCapabilities", ())
            .await
            .map_err(|_| "The iOS runner report is unavailable.")
    }
}

#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("runner")
        .setup(|app, api| {
            #[cfg(target_os = "ios")]
            {
                use tauri::Manager;
                let handle = api.register_ios_plugin(init_plugin_runner)?;
                app.manage(Runner(handle));
            }
            #[cfg(not(target_os = "ios"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
