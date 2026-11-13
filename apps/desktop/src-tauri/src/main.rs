//! Anima desktop shell: the main window and OS services only (REPO-003).
//! The UI is the static `apps/client` build; no business logic lives here.

mod agent;
mod builder;
mod keystore;
mod mcp_permissions;
mod os_program;
mod process_log;
mod system_browser;
mod system_settings;
mod this_machine;
mod web_view;

use tauri::Manager;

fn main() {
    let app = tauri::Builder::default().plugin(process_log::plugin());
    // Release desktop builds only: `tauri dev` never checks for or loads an update (UPD-001).
    #[cfg(all(desktop, not(debug_assertions)))]
    let app = app
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());
    app.manage(web_view::Tabs::default())
        .manage(builder::Running::default())
        .manage(builder::Bundled::default())
        .manage(agent::Installing::default())
        .setup(|_| {
            tauri::async_runtime::spawn(keystore::forget_retired_updater_key());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            keystore::access_token_read,
            keystore::access_token_write,
            keystore::access_token_clear,
            keystore::builder_token_read,
            keystore::builder_token_write,
            keystore::builder_token_clear,
            builder::builder_tools_check,
            builder::builder_checkout,
            builder::builder_gates_run,
            builder::builder_bundle,
            builder::builder_cancel,
            builder::upload::builder_release_files,
            builder::upload::builder_upload,
            mcp_permissions::mcp_permissions_read,
            mcp_permissions::mcp_permissions_write,
            process_log::process_log_folder_open,
            system_settings::sharing_settings_open,
            this_machine::this_machine_read,
            agent::agent_bundled,
            agent::agent_install,
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
        .run(|app, event| on_run_event(app, &event));
}

/// No build step or agent run outlives the app (BLD-026, MET-019).
fn on_run_event(app: &tauri::AppHandle, event: &tauri::RunEvent) {
    if matches!(event, tauri::RunEvent::Exit) {
        app.state::<builder::Running>().cancel();
        app.state::<agent::Installing>().stop();
    }
}
