//! Native inventory for the local runner report (OD-14).

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_runner);

#[cfg(any(target_os = "ios", target_os = "android"))]
pub struct Runner<R: Runtime>(tauri::plugin::PluginHandle<R>);

#[cfg(any(target_os = "ios", target_os = "android"))]
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
            .map_err(|_| "The native runner report is unavailable.")
    }
}

#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("runner")
        .setup(|app, api| {
            #[cfg(any(target_os = "ios", target_os = "android"))]
            {
                use tauri::Manager;
                #[cfg(target_os = "ios")]
                let handle = api.register_ios_plugin(init_plugin_runner)?;
                #[cfg(target_os = "android")]
                let handle = api.register_android_plugin("ing.anima.runner", "RunnerPlugin")?;
                app.manage(Runner(handle));
            }
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
