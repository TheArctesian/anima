//! The client-local read command for the runner report (OD-14).

use tauri::Manager;

#[tauri::command]
pub async fn runner_capabilities_read<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<serde_json::Value, &'static str> {
    app.state::<tauri_plugin_runner::Runner<R>>().read().await
}
