//! Anima desktop shell: the main window and OS services only (REPO-003).
//! The UI is the static `apps/client` build; no business logic lives here.

mod keystore;
mod os_program;
mod process_log;
mod system_browser;
mod system_settings;
mod this_machine;
mod web_view;

fn main() {
    let builder = tauri::Builder::default().plugin(process_log::plugin());
    // Release desktop builds only: `tauri dev` never checks for or loads an update (UPD-001).
    #[cfg(all(desktop, not(debug_assertions)))]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());
    builder
        .manage(web_view::Tabs::default())
        .invoke_handler(tauri::generate_handler![
            keystore::access_token_read,
            keystore::access_token_write,
            keystore::access_token_clear,
            process_log::process_log_folder_open,
            system_settings::sharing_settings_open,
            this_machine::this_machine_read,
            system_browser::system_browser_open,
            web_view::commands::web_view_pinning,
            web_view::commands::web_view_open,
            web_view::commands::web_view_place,
            web_view::commands::web_view_show,
            web_view::commands::web_view_go,
            web_view::commands::web_view_pin,
            web_view::commands::web_view_pop_out,
            web_view::commands::web_view_close,
            web_view::commands::web_certificate_read,
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri application failed to start");
}
