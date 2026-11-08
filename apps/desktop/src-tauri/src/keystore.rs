//! The hub tokens, one of each kind per hub origin, and the updater key, in the OS key store
//! (CLI-003, BLD-023, BLD-045, LOG-007). Device credentials never appear here (CLI-004): the hub
//! keeps them.

mod error;
mod signing_key;
mod token;

#[cfg(test)]
pub(crate) mod tests;

pub use error::KeystoreError;
pub use signing_key::{read_signing_key, write_signing_key};
use token::TokenKind;

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

/// This computer's builder token for the hub at `origin` (BLD-023).
#[tauri::command]
pub async fn builder_token_read(origin: String) -> Result<Option<String>, KeystoreError> {
    blocking(move || token::read_token(TokenKind::Builder, &origin)).await
}

#[tauri::command]
pub async fn builder_token_write(origin: String, token: String) -> Result<(), KeystoreError> {
    blocking(move || token::write_token(TokenKind::Builder, &origin, &token)).await
}

/// Deletes the builder token when the operator turns building off (BLD-025).
#[tauri::command]
pub async fn builder_token_clear(origin: String) -> Result<(), KeystoreError> {
    blocking(move || token::clear_token(TokenKind::Builder, &origin)).await
}
