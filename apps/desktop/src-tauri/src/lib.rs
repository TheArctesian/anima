//! SPIKE (not for merge): the mobile entry point. The window and the hub access token only.
#![cfg(mobile)]
#![allow(dead_code, reason = "spike")]

use std::collections::HashMap;
use std::sync::Mutex;

/// Android has no keyring backend in keyring 3.x; the spike keeps the token in memory there.
#[derive(Default)]
struct MemoryTokens(Mutex<HashMap<String, String>>);

#[cfg(any())]
const SERVICE: &str = "ing.anima.mobile.spike";

#[tauri::command]
fn access_token_read(
    origin: String,
    memory: tauri::State<'_, MemoryTokens>,
) -> Result<Option<String>, String> {
    #[cfg(any())]
    {
        let _ = &memory;
        let entry = keyring::Entry::new(SERVICE, &origin).map_err(|e| e.to_string())?;
        return match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        };
    }
    #[cfg(not(any()))]
    Ok(memory.0.lock().expect("token lock").get(&origin).cloned())
}

#[tauri::command]
fn access_token_write(
    origin: String,
    token: String,
    memory: tauri::State<'_, MemoryTokens>,
) -> Result<(), String> {
    #[cfg(any())]
    {
        let _ = &memory;
        return keyring::Entry::new(SERVICE, &origin)
            .and_then(|entry| entry.set_password(&token))
            .map_err(|e| e.to_string());
    }
    #[cfg(not(any()))]
    {
        memory.0.lock().expect("token lock").insert(origin, token);
        Ok(())
    }
}

#[tauri::command]
fn access_token_clear(
    origin: String,
    memory: tauri::State<'_, MemoryTokens>,
) -> Result<(), String> {
    #[cfg(any())]
    {
        let _ = &memory;
        return match keyring::Entry::new(SERVICE, &origin).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    #[cfg(not(any()))]
    {
        memory.0.lock().expect("token lock").remove(&origin);
        Ok(())
    }
}

#[tauri::mobile_entry_point]
pub fn run() {
    tauri::Builder::default()
        .manage(MemoryTokens::default())
        .invoke_handler(tauri::generate_handler![
            access_token_read,
            access_token_write,
            access_token_clear
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri application failed to start");
}
