//! The hub tokens, one of each kind per hub origin, in the OS key store (CLI-003, BLD-023,
//! LOG-007). Device credentials never appear here (CLI-004): the hub keeps them, and the updater
//! key never does: the hub holds it (R-117).

mod error;
mod retired;
mod token;

#[cfg(test)]
pub(crate) mod tests;

pub use error::KeystoreError;
use tauri::{Runtime, Webview};
use tauri_plugin_log::log;
use token::TokenKind;

use crate::web_view::MAIN;

/// Runs a blocking key-store call off the main thread: the OS may prompt the operator.
async fn blocking<T, F>(call: F) -> Result<T, KeystoreError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, KeystoreError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(call)
        .await
        .map_err(|_| KeystoreError::Failed)?
}

#[tauri::command]
pub async fn access_token_read(origin: String) -> Result<Option<String>, KeystoreError> {
    blocking(move || token::read_token(TokenKind::Access, &origin)).await
}

#[tauri::command]
pub async fn access_token_write(origin: String, token: String) -> Result<(), KeystoreError> {
    blocking(move || token::write_token(TokenKind::Access, &origin, &token)).await
}

#[tauri::command]
pub async fn access_token_clear(origin: String) -> Result<(), KeystoreError> {
    blocking(move || token::clear_token(TokenKind::Access, &origin)).await
}

/// Refuses every caller but the main web view; the capability already does, this is the second
/// wall (C-5).
fn from_main(label: &str) -> Result<(), KeystoreError> {
    (label == MAIN).then_some(()).ok_or(KeystoreError::Caller)
}

/// This computer's builder token for the hub at `origin` (BLD-023).
#[tauri::command]
pub async fn builder_token_read<R: Runtime>(
    webview: Webview<R>,
    origin: String,
) -> Result<Option<String>, KeystoreError> {
    from_main(webview.label())?;
    builder_read(origin).await
}

async fn builder_read(origin: String) -> Result<Option<String>, KeystoreError> {
    blocking(move || token::read_token(TokenKind::Builder, &origin)).await
}

#[tauri::command]
pub async fn builder_token_write<R: Runtime>(
    webview: Webview<R>,
    origin: String,
    token: String,
) -> Result<(), KeystoreError> {
    from_main(webview.label())?;
    builder_write(origin, token).await
}

async fn builder_write(origin: String, token: String) -> Result<(), KeystoreError> {
    blocking(move || token::write_token(TokenKind::Builder, &origin, &token)).await
}

/// Deletes the builder token when the operator turns building off (BLD-025).
#[tauri::command]
pub async fn builder_token_clear<R: Runtime>(
    webview: Webview<R>,
    origin: String,
) -> Result<(), KeystoreError> {
    from_main(webview.label())?;
    builder_clear(origin).await
}

async fn builder_clear(origin: String) -> Result<(), KeystoreError> {
    blocking(move || token::clear_token(TokenKind::Builder, &origin)).await
}

/// Deletes the updater key entries an earlier version imported (R-117, D-41), off the main
/// thread. Logs only the error code, never an entry (LOG-007).
pub async fn forget_retired_updater_key() {
    if let Err(error) = blocking(retired::forget_updater_key).await {
        log::warn!(
            "the retired updater key entries were not deleted: {}",
            error.code()
        );
    }
}
