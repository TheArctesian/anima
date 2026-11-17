//! The mobile application: the window and the hub access token (MOB-002).
//! This file wires; it holds no rules.

use crate::memory_tokens;

/// Starts the mobile application. iOS and Android call it; it returns when the application ends.
#[tauri::mobile_entry_point]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_keyboard::init())
        .manage(memory_tokens::MemoryTokens::default())
        .invoke_handler(tauri::generate_handler![
            memory_tokens::access_token_read,
            memory_tokens::access_token_write,
            memory_tokens::access_token_clear,
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri application failed to start");
}
