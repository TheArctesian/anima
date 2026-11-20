//! The mobile application: the window, the hub access token, the device link and the sign-in to the
//! Anima.ing account (MOB-002, MOB-031; mobile plan M3-4).
//! This file wires; it holds no rules.

use tauri::RunEvent;

use crate::{cloud, phone_browser, phone_link};

pub(crate) mod account_session;
mod runner;
pub(crate) mod tokens;

/// Starts the mobile application. iOS and Android call it; it returns when the application ends.
#[tauri::mobile_entry_point]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_runner::init())
        .plugin(tauri_plugin_keyboard::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_account_session::init())
        .manage(cloud::Cloud::default())
        .setup(|app| {
            phone_link::manage(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tokens::access_token_read,
            tokens::access_token_write,
            tokens::access_token_clear,
            phone_link::link_join,
            phone_link::link_status,
            phone_link::hub_listener_check,
            phone_link::link_leave,
            cloud::cloud_sign_in_start,
            cloud::cloud_sign_in_poll,
            cloud::cloud_sign_in_resume,
            cloud::cloud_sign_out,
            cloud::in_app::cloud_sign_in_session,
            cloud::cloud_hub,
            cloud::cloud_hub_access,
            phone_browser::system_browser_open,
            runner::runner_capabilities_read,
        ])
        .build(tauri::generate_context!())
        .expect("the Tauri application failed to start")
        .run(|app, event| {
            if matches!(event, RunEvent::Resumed) {
                phone_link::resume(app);
            }
        });
}
