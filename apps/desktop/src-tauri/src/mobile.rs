//! The mobile application: the window, the hub access token, the device link and the sign-in to the
//! Anima.ing account (MOB-002, MOB-031; mobile plan M3-4).
//! This file wires; it holds no rules.

use tauri::RunEvent;

use crate::{cloud, memory_tokens, phone_browser, phone_link};

/// Starts the mobile application. iOS and Android call it; it returns when the application ends.
#[tauri::mobile_entry_point]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_keyboard::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_account_session::init())
        .manage(memory_tokens::MemoryTokens::default())
        .manage(cloud::Cloud::default())
        .setup(|app| {
            phone_link::manage(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            memory_tokens::access_token_read,
            memory_tokens::access_token_write,
            memory_tokens::access_token_clear,
            phone_link::link_join,
            phone_link::link_status,
            phone_link::link_leave,
            cloud::cloud_sign_in_start,
            cloud::cloud_sign_in_poll,
            cloud::cloud_sign_in_resume,
            cloud::in_app::cloud_sign_in_session,
            cloud::cloud_hub,
            cloud::cloud_hub_access,
            phone_browser::system_browser_open,
        ])
        .build(tauri::generate_context!())
        .expect("the Tauri application failed to start")
        .run(|app, event| {
            if matches!(event, RunEvent::Resumed) {
                phone_link::resume(app);
            }
        });
}
