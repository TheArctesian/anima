//! Anima desktop shell: the main window and OS services only (REPO-003).
//! The UI is the static `apps/client` build; no business logic lives here.

mod keystore;
mod process_log;
mod system_settings;

fn main() {
    let builder = tauri::Builder::default().plugin(process_log::plugin());
    // Release desktop builds only: `tauri dev` never checks for or loads an update (UPD-001).
    #[cfg(all(desktop, not(debug_assertions)))]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());
    builder
        .invoke_handler(tauri::generate_handler![
            keystore::access_token_read,
            keystore::access_token_write,
            keystore::access_token_clear,
            process_log::process_log_folder_open,
            system_settings::remote_login_settings_open,
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri application failed to start");
}
