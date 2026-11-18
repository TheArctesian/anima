//! The mobile application: the window, the hub access token and the device link (MOB-002, MOB-031).
//! This file wires; it holds no rules.

use tauri::RunEvent;

use crate::{memory_tokens, phone_link};

/// Starts the mobile application. iOS and Android call it; it returns when the application ends.
#[tauri::mobile_entry_point]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_keyboard::init())
        .manage(memory_tokens::MemoryTokens::default())
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
        ])
        .build(tauri::generate_context!())
        .expect("the Tauri application failed to start")
        .run(|app, event| {
            if matches!(event, RunEvent::Resumed) {
                phone_link::resume(app);
            }
        });
}
