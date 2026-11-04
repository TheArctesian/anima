//! The hub access token, one per hub origin, in the OS key store (CLI-003, LOG-007). Device credentials never
//! appear here (CLI-004): the hub keeps them.

mod error;
mod token;

#[cfg(test)]
mod tests;

pub use error::KeystoreError;

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
    blocking(move || token::read_token(&origin)).await
}

#[tauri::command]
pub async fn access_token_write(origin: String, token: String) -> Result<(), KeystoreError> {
    blocking(move || token::write_token(&origin, &token)).await
}

#[tauri::command]
pub async fn access_token_clear(origin: String) -> Result<(), KeystoreError> {
    blocking(move || token::clear_token(&origin)).await
}
