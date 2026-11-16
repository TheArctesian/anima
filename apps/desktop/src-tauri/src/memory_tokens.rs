//! The hub access tokens of a mobile development build, in memory (OD-M7 of the mobile plan).
//! A token is lost when the application stops. A build that leaves the operator's computers
//! must keep the token in the key store of the operating system instead (MOB-006).
//! Nothing here logs a token (LOG-007).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::State;

/// One access token for each hub origin.
#[derive(Default)]
pub struct MemoryTokens(Mutex<HashMap<String, String>>);

impl MemoryTokens {
    /// The tokens. A panic in another command does not make them unreadable.
    fn tokens(&self) -> MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The access token kept for `origin`, or `None`. A token is only read back for the hub that
/// issued it (CLI-002).
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Tauri command takes its arguments by value"
)]
pub fn access_token_read(origin: String, kept: State<'_, MemoryTokens>) -> Option<String> {
    kept.tokens().get(&origin).cloned()
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Tauri command takes its arguments by value"
)]
pub fn access_token_write(origin: String, token: String, kept: State<'_, MemoryTokens>) {
    kept.tokens().insert(origin, token);
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Tauri command takes its arguments by value"
)]
pub fn access_token_clear(origin: String, kept: State<'_, MemoryTokens>) {
    kept.tokens().remove(&origin);
}
