//! The updater key on a builder computer (BLD-045, BLD-046, UPD-012, D-19). The operator imports
//! the key file once; the OS key store keeps it sealed with its password, never as a bare seed,
//! because any process of the operator's user can read that store. To build, the operator
//! unlocks it once per app session: the opened key lives only in this process's memory, and
//! locking, turning building off or quitting drops it. Files are signed in this process.
//!
//! The web view learns only whether a key is stored, whether it is unlocked, its public key line
//! and whether that matches the public key this app was built with.

mod error;
mod sealed;
mod secret;
mod signature;
mod unlocked;

#[cfg(test)]
mod tests;

use std::{fs::File, io::Read, path::Path};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State, Webview};
use tauri_plugin_dialog::DialogExt;
use zeroize::Zeroizing;

use crate::{keystore, web_view::MAIN};
pub use error::UpdaterKeyError;
use secret::PublicKey;
pub(crate) use signature::{Signed, prehash};
#[cfg(target_os = "linux")]
pub use unlocked::keep_out_of_core_dumps;
pub use unlocked::{UnlockedKey, dumpable};

/// A key file is a few hundred bytes; anything larger is not one.
const KEY_FILE_MAX_BYTES: u64 = 4096;

/// What the Settings row shows about the updater key (BLD-020, BLD-028).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterKeyState {
    pub present: bool,
    /// Whether the key is open in this app session, so the builder may sign.
    pub unlocked: bool,
    /// The key line of the public key (`RWQ...`), when a key is stored.
    pub public_key: Option<String>,
    /// Whether that is the public key in this app's `tauri.conf.json`; `None` without a key.
    pub matches_app: Option<bool>,
}

/// The key line of the base64 public key file in `plugins.updater.pubkey`, if it has one.
pub(crate) fn app_public_key<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let encoded = app
        .config()
        .plugins
        .0
        .get("updater")?
        .get("pubkey")?
        .as_str()?;
    key_line(encoded)
}

/// The last line of the public key file whose base64 is `encoded`, if it is text.
pub(crate) fn key_line(encoded: &str) -> Option<String> {
    let text = String::from_utf8(STANDARD.decode(encoded).ok()?).ok()?;
    text.lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(str::to_owned)
}

fn state_of(public_key: Option<String>, unlocked: bool, app_key: Option<&str>) -> UpdaterKeyState {
    UpdaterKeyState {
        present: public_key.is_some(),
        unlocked: unlocked && public_key.is_some(),
        matches_app: public_key.as_deref().map(|line| Some(line) == app_key),
        public_key,
    }
}

/// Refuses every caller but the main web view; the capability already does, this is the second
/// wall (C-5).
fn from_main<R: Runtime>(webview: &Webview<R>) -> Result<(), UpdaterKeyError> {
    (webview.label() == MAIN)
        .then_some(())
        .ok_or(UpdaterKeyError::Caller)
}

async fn off_main<T: Send + 'static>(
    call: impl FnOnce() -> Result<T, UpdaterKeyError> + Send + 'static,
) -> Result<T, UpdaterKeyError> {
    tauri::async_runtime::spawn_blocking(call)
        .await
        .map_err(|_| UpdaterKeyError::Store(keystore::KeystoreError::Failed))?
}

/// Whether a key is stored and unlocked, and its public key. Never reads the sealed key.
#[tauri::command]
pub async fn updater_key_read<R: Runtime>(
    webview: Webview<R>,
    unlocked: State<'_, UnlockedKey>,
) -> Result<UpdaterKeyState, UpdaterKeyError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let app_key = app_public_key(&app);
    let open = unlocked.is_unlocked();
    off_main(move || {
        let public = keystore::read_signing_public_key()?;
        Ok(state_of(public, open, app_key.as_deref()))
    })
    .await
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

/// Opens a native file dialog and returns the chosen path; the web view never names it.
async fn chosen_file<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<std::path::PathBuf, UpdaterKeyError> {
    let (chosen, picked) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose the updater key file")
        .pick_file(move |path| {
            let _ = chosen.send(path);
        });
    picked
        .await
        .ok()
        .flatten()
        .and_then(|path| path.into_path().ok())
        .ok_or(UpdaterKeyError::Cancelled)
}

/// Reads the minisign secret key file the operator picks, checks `password` opens it, and stores
/// it sealed (BLD-045, UPD-012, D-19): as it is when it has a password and scrypt `N` of 2^15 or
/// more, sealed anew otherwise (see `sealed::imported`). The key is then unlocked for this app
/// session.
#[tauri::command]
pub async fn updater_key_import<R: Runtime>(
    webview: Webview<R>,
    unlocked: State<'_, UnlockedKey>,
    password: Option<String>,
    new_password: Option<String>,
) -> Result<UpdaterKeyState, UpdaterKeyError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let password = password.map(Zeroizing::new);
    let new_password = new_password.map(Zeroizing::new);
    let path = chosen_file(&app).await?;
    let app_key = app_public_key(&app);
    let imported = off_main(move || {
        let file = read_key_file(&path)?;
        let mut salt = [0; 32];
        getrandom::fill(&mut salt).map_err(|_| UpdaterKeyError::Signature)?;
        let imported = sealed::imported(
            &file,
            password.as_deref().map(String::as_str),
            new_password.as_deref().map(String::as_str),
            &salt,
        )?;
        let public = imported.key.public_key().line();
        keystore::write_signing_key(&imported.stored, &public)?;
        Ok(imported.key)
    })
    .await?;
    let public = imported.public_key().line();
    unlocked.set(imported);
    Ok(state_of(Some(public), true, app_key.as_deref()))
}

/// Opens the stored key with `password` and keeps it in this process's memory until it is
/// locked or the app quits (D-19). A wrong password is refused and leaves the key locked, and so
/// is a key whose public key is not the line stored beside it.
#[tauri::command]
pub async fn updater_key_unlock<R: Runtime>(
    webview: Webview<R>,
    unlocked: State<'_, UnlockedKey>,
    password: String,
) -> Result<UpdaterKeyState, UpdaterKeyError> {
    from_main(&webview)?;
    let app = webview.app_handle().clone();
    let password = Zeroizing::new(password);
    let app_key = app_public_key(&app);
    let key = off_main(move || {
        let stored = keystore::read_signing_key()?.ok_or(UpdaterKeyError::Missing)?;
        let public = keystore::read_signing_public_key()?.ok_or(UpdaterKeyError::Missing)?;
        sealed::unlocked(&stored, &password, &public)
    })
    .await?;
    let public = key.public_key().line();
    unlocked.set(key);
    Ok(state_of(Some(public), true, app_key.as_deref()))
}

/// Drops the opened key from memory (D-19). The sealed key stays in the key store.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its state by value"
)]
#[tauri::command]
pub fn updater_key_lock<R: Runtime>(
    webview: Webview<R>,
    unlocked: State<'_, UnlockedKey>,
) -> Result<(), UpdaterKeyError> {
    from_main(&webview)?;
    unlocked.lock();
    Ok(())
}

/// The stored public key, for checking a signature without the secret key (C-2). Blocking.
pub(crate) fn stored_public_key() -> Result<PublicKey, UpdaterKeyError> {
    let line = keystore::read_signing_public_key()?.ok_or(UpdaterKeyError::Missing)?;
    PublicKey::from_line(&line)
}

/// Checks a `.sig` file text over `file` with the stored public key and `version` (BLD-046).
pub(crate) fn verify_file(
    file: impl Read,
    signature_file: &str,
    version: &str,
) -> Result<(), UpdaterKeyError> {
    signature::verify(&stored_public_key()?, file, signature_file, version)
}

/// Whether a key is stored, for the tool check (BLD-028). Blocking; reads only the public line.
pub(crate) fn is_present() -> Result<bool, UpdaterKeyError> {
    Ok(keystore::read_signing_public_key()?.is_some())
}
