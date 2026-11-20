//! Windows client entry point (WIN1). Unix host installation and build commands are not registered.
use crate::{
    cloud, hub_listener, keystore, link, process_log, system_browser, this_machine, web_view,
};
use std::sync::Arc;
use tauri::Manager;

/// Starts the Windows desktop client with the shared account, device and session UI.
pub fn run() {
    tauri::Builder::default()
        .plugin(process_log::plugin())
        .plugin(tauri_plugin_dialog::init())
        .manage(web_view::Tabs::default())
        .manage(cloud::Cloud::default())
        .manage(link::Changing::default())
        .manage(Arc::new(link::DeviceLink::default()))
        .setup(|app| {
            tauri::async_runtime::spawn(keystore::forget_retired_updater_key());
            link::start(&app.state::<Arc<link::DeviceLink>>());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            process_log::process_log_folder_open,
            this_machine::this_machine_read,
            system_browser::system_browser_open,
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
        ])
        .build(tauri::generate_context!())
        .expect("the Windows application failed to start")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<Arc<link::DeviceLink>>().stop();
            }
        });
}
