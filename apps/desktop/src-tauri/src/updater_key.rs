//! The updater key on a builder computer (BLD-045, BLD-046, UPD-012). The operator imports the
//! key file once; it goes into the OS key store and never comes back to the web view, to a child
//! process, to a file or to the hub. Files are signed in this process.
//!
//! The web view learns only whether a key is stored, its public key line and whether that matches
//! the public key this app was built with.

mod error;
mod secret;
mod signature;

#[cfg(test)]
mod tests;

use std::{fs::File, io::Read, path::Path};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;
use zeroize::Zeroizing;

use crate::keystore;
pub use error::UpdaterKeyError;
use secret::SecretKey;
pub(crate) use signature::Signed;

/// A key file is a few hundred bytes; anything larger is not one.
const KEY_FILE_MAX_BYTES: u64 = 4096;

/// What the Settings row shows about the updater key (BLD-020, BLD-028).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterKeyState {
    pub present: bool,
    /// The key line of the public key (`RWQ...`), when a key is stored.
    pub public_key: Option<String>,
    /// Whether that is the public key in this app's `tauri.conf.json`; `None` without a key.
    pub matches_app: Option<bool>,
}

/// The key line of the base64 public key file in `plugins.updater.pubkey`, if it has one.
fn app_public_key<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let encoded = app
        .config()
        .plugins
        .0
        .get("updater")?
        .get("pubkey")?
        .as_str()?;
    let text = String::from_utf8(STANDARD.decode(encoded).ok()?).ok()?;
    text.lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(str::to_owned)
}

fn state_of(key: Option<&SecretKey>, app_key: Option<&str>) -> UpdaterKeyState {
    let public_key = key.map(|key| key.public_key().line());
    UpdaterKeyState {
        present: key.is_some(),
        matches_app: public_key.as_deref().map(|line| Some(line) == app_key),
        public_key,
    }
}

/// The stored key, or `None`. A stored text that is not a key reads as `Format`.
fn stored_key() -> Result<Option<SecretKey>, UpdaterKeyError> {
    keystore::read_signing_key()?
        .map(|text| SecretKey::from_stored(&text))
        .transpose()
}

async fn off_main<T: Send + 'static>(
    call: impl FnOnce() -> Result<T, UpdaterKeyError> + Send + 'static,
) -> Result<T, UpdaterKeyError> {
    tauri::async_runtime::spawn_blocking(call)
        .await
        .map_err(|_| UpdaterKeyError::Store(keystore::KeystoreError::Failed))?
}

/// Whether a key is stored, and its public key.
#[tauri::command]
pub async fn updater_key_read<R: Runtime>(
    app: AppHandle<R>,
) -> Result<UpdaterKeyState, UpdaterKeyError> {
    let app_key = app_public_key(&app);
    off_main(move || Ok(state_of(stored_key()?.as_ref(), app_key.as_deref()))).await
}

/// Reads at most `KEY_FILE_MAX_BYTES` of the file the operator chose.
fn read_key_file(path: &Path) -> Result<Zeroizing<Vec<u8>>, UpdaterKeyError> {
    let file = File::open(path).map_err(|_| UpdaterKeyError::File)?;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(KEY_FILE_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| UpdaterKeyError::File)?;
    if bytes.len() as u64 > KEY_FILE_MAX_BYTES {
        return Err(UpdaterKeyError::Format);
    }
    Ok(bytes)
}

/// Opens a native file dialog, reads the chosen minisign secret key file, opens it with
/// `password` when it is encrypted, and stores the key. The web view never names the file and
/// never sees the key (BLD-045, UPD-012).
#[tauri::command]
pub async fn updater_key_import<R: Runtime>(
    app: AppHandle<R>,
    password: Option<String>,
) -> Result<UpdaterKeyState, UpdaterKeyError> {
    let password = password.map(Zeroizing::new);
    let (chosen, picked) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose the updater key file")
        .pick_file(move |path| {
            let _ = chosen.send(path);
        });
    let path = picked
        .await
        .ok()
        .flatten()
        .and_then(|path| path.into_path().ok())
        .ok_or(UpdaterKeyError::Cancelled)?;
    let app_key = app_public_key(&app);
    off_main(move || {
        let file = read_key_file(&path)?;
        let key = SecretKey::from_key_file(&file, password.as_deref().map(String::as_str))?;
        keystore::write_signing_key(&key.to_stored())?;
        Ok(state_of(Some(&key), app_key.as_deref()))
    })
    .await
}

/// Signs `file` with the stored key and checks the result as the updater will (BLD-045,
/// BLD-046). Blocking: call it off the main thread. Returns the `.sig` file text.
pub(crate) fn sign_file(path: &Path, signed: &Signed<'_>) -> Result<String, UpdaterKeyError> {
    let key = stored_key()?.ok_or(UpdaterKeyError::Missing)?;
    let file = File::open(path).map_err(|_| UpdaterKeyError::File)?;
    let signature_file = signature::sign(&key, file, signed)?;
    let again = File::open(path).map_err(|_| UpdaterKeyError::File)?;
    signature::verify(&key.public_key(), again, &signature_file, signed.version)?;
    Ok(signature_file)
}

/// Checks a `.sig` file text over `path` with the stored key's public key and `version`
/// (BLD-046). Blocking.
pub(crate) fn verify_file(
    path: &Path,
    signature_file: &str,
    version: &str,
) -> Result<(), UpdaterKeyError> {
    let key = stored_key()?.ok_or(UpdaterKeyError::Missing)?;
    let file = File::open(path).map_err(|_| UpdaterKeyError::File)?;
    signature::verify(&key.public_key(), file, signature_file, version)
}

/// Whether a key is stored, for the tool check (BLD-028). Blocking.
pub(crate) fn is_present() -> Result<bool, UpdaterKeyError> {
    Ok(stored_key()?.is_some())
}
