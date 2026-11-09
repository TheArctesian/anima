//! Anima desktop shell: the main window and OS services only (REPO-003).
//! The UI is the static `apps/client` build; no business logic lives here.

mod builder;
mod keystore;
mod os_program;
mod process_log;
mod system_browser;
mod system_settings;
mod this_machine;
mod updater_key;
mod web_view;

use tauri::Manager;

fn main() {
    // The dialog plugin is used from Rust only (the updater key file); no capability names it.
    let app = tauri::Builder::default()
        .plugin(process_log::plugin())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_| {
            // Before any key is opened: no core dump of this process may hold it (D-19).
            #[cfg(target_os = "linux")]
            if let Err(error) = updater_key::keep_out_of_core_dumps() {
                tauri_plugin_log::log::warn!("this process is still dumpable: {error}");
            }
            Ok(())
        });
    // Release desktop builds only: `tauri dev` never checks for or loads an update (UPD-001).
    #[cfg(all(desktop, not(debug_assertions)))]
    let app = app
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());
    app.manage(web_view::Tabs::default())
        .manage(builder::Running::default())
        .manage(builder::Bundled::default())
        .manage(updater_key::UnlockedKey::default())
        .invoke_handler(tauri::generate_handler![
            keystore::access_token_read,
            keystore::access_token_write,
            keystore::access_token_clear,
            keystore::builder_token_read,
            keystore::builder_token_write,
            keystore::builder_token_clear,
            updater_key::updater_key_read,
            updater_key::updater_key_import,
            updater_key::updater_key_unlock,
            updater_key::updater_key_lock,
            builder::builder_tools_check,
            builder::builder_checkout,
            builder::builder_gates_run,
            builder::builder_bundle,
            builder::builder_sign,
            builder::builder_signature_verify,
            builder::builder_cancel,
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
        .build(tauri::generate_context!())
        .expect("the Tauri application failed to start")
        .run(|app, event| {
            // No build step outlives the app (BLD-026), and the opened key goes with it (D-19).
            if let tauri::RunEvent::Exit = event {
                app.state::<builder::Running>().cancel();
                app.state::<updater_key::UnlockedKey>().lock();
            }
        });
}
