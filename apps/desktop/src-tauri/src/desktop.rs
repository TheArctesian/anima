//! The desktop application: the main window and the OS services of macOS and Linux (REPO-003).
//! This file wires; it holds no rules.

use std::sync::Arc;

use tauri::Manager;

use crate::{
    agent_runner, agent_threads, builder, cloud, command_line, daemon, hub_listener, keystore,
    link, local_hub, mcp_agents, mcp_permissions, process_log, stay_awake, system_browser,
    system_settings, this_machine, web_view,
};

/// Starts the desktop application and returns when it ends.
pub fn run() {
    // An AppImage started by the `anima` launcher runs the command line, not the window (INS-005).
    command_line::dispatch::run_if_asked();
    // The dialog plugin's JavaScript API is granted to no web view: only Rust shows its dialogs.
    let app = tauri::Builder::default()
        .plugin(process_log::plugin())
        .plugin(tauri_plugin_dialog::init())
        // Rust only: the page asks the shell, which shows the notification of a thread (THR-059).
        .plugin(tauri_plugin_notification::init());
    // Release desktop builds only: `tauri dev` never checks for or loads an update (UPD-001).
    #[cfg(not(debug_assertions))]
    let app = with_updater(app).plugin(tauri_plugin_process::init());
    with_state(app)
        .invoke_handler(commands())
        .build(tauri::generate_context!())
        .expect("the Tauri application failed to start")
        .run(|app, event| on_run_event(app, &event));
}

/// Adds the updater, unless `trial-bundle.sh` built this program (`ANIMA_LOCAL_BUILD`, read when
/// the program is compiled). A trial bundle has no updater: the published version can be lower
/// than its own, and the updater would put it in its place (UPD-001, DEF-001).
#[cfg(not(debug_assertions))]
fn with_updater(app: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    if option_env!("ANIMA_LOCAL_BUILD").is_some() {
        return app;
    }
    app.plugin(tauri_plugin_updater::Builder::new().build())
}

/// The shell's commands; each refuses every web view but the main one (WEB-007).
#[expect(
    clippy::too_many_lines,
    reason = "a list with one line for each command of the shell; it holds no logic"
)]
fn commands() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        keystore::access_token_read,
        cloud::cloud_sign_in_start,
        cloud::cloud_sign_in_poll,
        cloud::cloud_sign_in_resume,
        cloud::cloud_hub,
        cloud::cloud_hub_access,
        cloud::cloud_device,
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
        agent_threads::agent_threads_list,
        agent_threads::agent_thread_read,
        agent_runner::agent_runner_call,
        agent_runner::agent_runner_watch,
        agent_runner::agent_thread_notify,
        mcp_agents::mcp_agents_read,
        mcp_agents::mcp_agents_change,
        mcp_permissions::mcp_permissions_read,
        mcp_permissions::mcp_permissions_write,
        process_log::process_log_folder_open,
        system_settings::sharing_settings_open,
        this_machine::this_machine_read,
        daemon::daemon_bundled,
        daemon::daemon_install,
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
        command_line::command_line_read,
        command_line::command_line_install,
        command_line::command_line_remove,
        system_browser::system_browser_open,
        stay_awake::stay_awake_read,
        stay_awake::stay_awake_choose,
        stay_awake::stay_awake_floor,
        stay_awake::stay_awake_lid_set_up,
        stay_awake::stay_awake_lid_remove,
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

/// Gives the application the state that its commands share, and what runs once at its start.
fn with_state(app: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    app.manage(web_view::Tabs::default())
        .manage(agent_runner::Watching::default())
        .manage(cloud::Cloud::default())
        .manage(builder::Running::default())
        .manage(builder::Bundled::default())
        .manage(daemon::Installing::default())
        .manage(local_hub::Changing::default())
        .manage(local_hub::Replacements::default())
        .manage(link::Changing::default())
        .manage(Arc::new(link::DeviceLink::default()))
        .setup(|app| {
            tauri::async_runtime::spawn(keystore::forget_retired_updater_key());
            stay_awake::start(app.handle());
            link::start(&app.state::<Arc<link::DeviceLink>>());
            local_hub::update::at_start(&app.state::<local_hub::Changing>());
            Ok(())
        })
}

/// No build step, agent run, device link or stay-awake hold outlives the app (BLD-026,
/// MET-019, AWK-007, S6 2.6).
fn on_run_event(app: &tauri::AppHandle, event: &tauri::RunEvent) {
    if matches!(event, tauri::RunEvent::Exit) {
        app.state::<builder::Running>().cancel();
        app.state::<daemon::Installing>().stop();
        app.state::<Arc<link::DeviceLink>>().stop();
        if let Some(stay_awake) = app.try_state::<stay_awake::StayAwake>() {
            stay_awake.release();
        }
    }
}
