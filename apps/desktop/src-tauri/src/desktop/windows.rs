//! Windows client entry point (WIN1). Unix host installation and build commands are not registered.
use crate::{
    builder, cloud, command_line, daemon, hub_listener, keystore, link, local_hub, mcp_agents,
    mcp_permissions, process_log, stay_awake, system_browser, this_machine, web_view,
};
use std::sync::Arc;
use tauri::Manager;

/// Starts the Windows desktop client with the shared account, device and session UI.
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(process_log::plugin())
        .plugin(tauri_plugin_dialog::init());
    #[cfg(not(debug_assertions))]
    let app = with_updater(app).plugin(tauri_plugin_process::init());
    app.manage(builder::Running::default())
        .manage(builder::Bundled::default())
        .manage(web_view::Tabs::default())
        .manage(cloud::Cloud::default())
        .manage(daemon::Installing::default())
        .manage(local_hub::Changing::default())
        .manage(local_hub::Replacements::default())
        .manage(link::Changing::default())
        .manage(Arc::new(link::DeviceLink::default()))
        .setup(|app| {
            fit_screen(app.handle());
            tauri::async_runtime::spawn(keystore::forget_retired_updater_key());
            stay_awake::start(app.handle());
            local_hub::update::at_start(&app.state::<local_hub::Changing>());
            link::start(&app.state::<Arc<link::DeviceLink>>());
            Ok(())
        })
        .invoke_handler(commands())
        .build(tauri::generate_context!())
        .expect("the Windows application failed to start")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<builder::Running>().cancel();
                app.state::<Arc<link::DeviceLink>>().stop();
                app.state::<daemon::Installing>().stop();
                if let Some(stay_awake) = app.try_state::<stay_awake::StayAwake>() {
                    stay_awake.release();
                }
            }
        });
}

/// Native Windows commands, granted only to the main webview.
#[expect(
    clippy::too_many_lines,
    reason = "one line per registered shell command, with no business logic"
)]
fn commands() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        builder::builder_tools_check,
        builder::builder_checkout,
        builder::builder_gates_run,
        builder::builder_bundle,
        builder::builder_cancel,
        builder::upload::builder_release_files,
        builder::upload::builder_upload,
        keystore::builder_token_read,
        keystore::builder_token_write,
        keystore::builder_token_clear,
        keystore::access_token_read,
        keystore::access_token_write,
        keystore::access_token_clear,
        cloud::cloud_sign_in_start,
        cloud::cloud_sign_in_poll,
        cloud::cloud_sign_in_resume,
        cloud::cloud_hub,
        cloud::cloud_hub_access,
        cloud::cloud_device,
        stay_awake::stay_awake_read,
        stay_awake::stay_awake_choose,
        stay_awake::stay_awake_floor,
        stay_awake::stay_awake_lid_set_up,
        stay_awake::stay_awake_lid_remove,
        daemon::daemon_bundled,
        daemon::daemon_install,
        daemon::daemon_remove,
        command_line::command_line_read,
        command_line::command_line_install,
        command_line::command_line_remove,
        mcp_agents::mcp_agents_read,
        mcp_agents::mcp_agents_change,
        mcp_permissions::mcp_permissions_read,
        mcp_permissions::mcp_permissions_write,
        process_log::process_log_folder_open,
        this_machine::this_machine_read,
        system_browser::system_browser_open,
        local_hub::local_hub_carried,
        local_hub::local_hub_install,
        local_hub::local_hub_remove,
        local_hub::local_hub_status,
        local_hub::local_hub_start,
        local_hub::local_hub_stop,
        local_hub::account::local_hub_account,
        local_hub::account::local_hub_totp_new,
        local_hub::account::local_hub_account_create,
        local_hub::account::local_hub_key_save,
        link::link_join,
        link::link_status,
        link::link_leave,
        hub_listener::hub_listener_check,
        web_view::commands::web_view_pinning,
        web_view::commands::web_view_open,
        web_view::commands::web_view_place,
        web_view::commands::web_view_show,
        web_view::commands::web_view_go,
        web_view::commands::web_view_pin,
        web_view::commands::web_view_pop_out,
        web_view::commands::web_view_close,
        web_view::commands::web_certificate_read,
    ]
}

/// Local trial builds never replace themselves with a published release (UPD-001).
#[cfg(not(debug_assertions))]
fn with_updater(app: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    if option_env!("ANIMA_LOCAL_BUILD").is_some() {
        return app;
    }
    app.plugin(tauri_plugin_updater::Builder::new().build())
}

/// Keep navigation reachable when the default window is larger than a VM or laptop screen.
fn fit_screen(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(web_view::MAIN) else {
        return;
    };
    let (Ok(size), Ok(Some(monitor))) = (window.outer_size(), window.current_monitor()) else {
        return;
    };
    if size.width > monitor.size().width || size.height > monitor.size().height {
        let _ = window.maximize();
    }
}
