//! The operator account of the hub on this computer (F-4, F-5, F-9) and the copy of its key
//! (2.8). The shell runs `anima-local-hub account` with each secret on standard input; it never
//! logs what the program reads or prints, only a fixed code and the exit code (LOG-007).
//!
//! `create` is permitted only while the hub has no account (the first run), or once after the
//! person at this computer confirmed recovery in a native dialog, which script in the web view
//! cannot answer (F-9).

use std::{
    ffi::OsString,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use serde::Deserialize;
use tauri::{AppHandle, Runtime, State, Webview};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_log::log;
use zeroize::{Zeroize, Zeroizing};

use super::{
    Changing, LocalHubError,
    account_form::{self, Account, NewTotp},
    carried, environment, from_main, key_copy,
    recovery::{self, CONFIRMATION, CONFIRMATION_TITLE, Recovery},
};
use crate::carried::run::{ExchangeError, Invocation, exchange};

/// `account create` hashes the password and writes the database.
const ACCOUNT_LIMIT: Duration = Duration::from_mins(1);
const KEY_FILE_NAME: &str = "anima-hub-key";

/// The recovery permit of this app (F-9).
#[derive(Debug, Default)]
pub struct RecoveryPermit(Mutex<Recovery>);

impl RecoveryPermit {
    fn with<T>(&self, work: impl FnOnce(&mut Recovery) -> T) -> T {
        work(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// What the client sends to make the account. Its secrets are wiped when it is dropped.
#[derive(Deserialize)]
pub struct NewAccount {
    name: String,
    password: String,
    secret: String,
    code: String,
}

impl Drop for NewAccount {
    fn drop(&mut self) {
        self.password.zeroize();
        self.secret.zeroize();
        self.code.zeroize();
    }
}

/// Runs `anima-local-hub account <args>`; a failed exit is `exited`. Logs only the verb, a
/// fixed code and the exit code.
fn run_account(
    args: &[&OsString],
    input: Option<&[u8]>,
    exited: LocalHubError,
) -> Result<Zeroizing<Vec<u8>>, LocalHubError> {
    let carried = carried()?;
    let env = environment(|name| std::env::var_os(name))?;
    let invocation = Invocation {
        program: &carried.program,
        args,
        env: &env,
        limit: ACCOUNT_LIMIT,
    };
    exchange(&invocation, input).map_err(|error| {
        let (answer, code) = match &error {
            ExchangeError::Exited { code, .. } => (exited, *code),
            ExchangeError::TimedOut => (LocalHubError::TimedOut, None),
            _ => (LocalHubError::Failed, None),
        };
        let verb = args
            .get(1)
            .map_or("?".into(), |verb| verb.to_string_lossy());
        log::warn!(
            "anima-local-hub account {verb}: {} ({error}; exit code {code:?})",
            answer.code()
        );
        answer
    })
}

fn status() -> Result<Option<Account>, LocalHubError> {
    let printed = run_account(
        &[&"account".into(), &"status".into()],
        None,
        LocalHubError::Failed,
    )?;
    account_form::parse_account(&printed)
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, LocalHubError> + Send + 'static,
) -> Result<T, LocalHubError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(LocalHubError::Failed))
}

/// The operator account of the hub on this computer: `{ name, totp }`, or `null` before the
/// first run made it.
#[tauri::command]
pub async fn local_hub_account<R: Runtime>(
    webview: Webview<R>,
) -> Result<Option<Account>, LocalHubError> {
    from_main(webview.label())?;
    blocking(status).await
}

/// A new TOTP secret for `name` and its `otpauth://` URI. The hub saves nothing yet.
#[tauri::command]
pub async fn local_hub_totp_new<R: Runtime>(
    webview: Webview<R>,
    name: String,
) -> Result<NewTotp, LocalHubError> {
    from_main(webview.label())?;
    let name = account_form::name(&name)?;
    blocking(move || {
        let printed = run_account(
            &[&"account".into(), &"totp-new".into(), &name],
            None,
            LocalHubError::Failed,
        )?;
        account_form::parse_new_totp(&printed)
    })
    .await
}

/// Makes the operator account, or replaces it after a confirmed recovery, with a password, a
/// TOTP secret and a code of that secret. Every sign-in to the hub ends.
#[tauri::command]
pub async fn local_hub_account_create<R: Runtime>(
    webview: Webview<R>,
    changing: State<'_, Changing>,
    permit: State<'_, RecoveryPermit>,
    account: NewAccount,
) -> Result<(), LocalHubError> {
    from_main(webview.label())?;
    let name = account_form::name(&account.name)?;
    let input = account_form::create_input(&account.password, &account.secret, &account.code)?;
    drop(account);
    let claim = changing.claim()?;
    let held = permit.with(|recovery| recovery.is_held(Instant::now()));
    let result = blocking(move || {
        let _claim = claim;
        recovery::may_create(status()?.is_some(), held)?;
        let args: [&OsString; 3] = [&"account".into(), &"create".into(), &name];
        run_account(&args, Some(&input), LocalHubError::Refused).map(drop)
    })
    .await;
    match &result {
        Ok(()) => {
            permit.with(Recovery::consume);
            log::info!("the operator account of the hub on this computer was made");
        }
        Err(error) => log::warn!("the operator account was not made: {}", error.code()),
    }
    result
}

/// Asks the person at this computer, in a native dialog, to confirm recovery of the account.
/// On "Recover" one `local_hub_account_create` may replace the account for five minutes.
#[tauri::command]
pub async fn local_hub_recover<R: Runtime>(
    app: AppHandle<R>,
    webview: Webview<R>,
    permit: State<'_, RecoveryPermit>,
) -> Result<bool, LocalHubError> {
    from_main(webview.label())?;
    let window = webview.window();
    let confirmed = blocking(move || {
        Ok(app
            .dialog()
            .message(CONFIRMATION)
            .title(CONFIRMATION_TITLE)
            .kind(MessageDialogKind::Warning)
            .parent(&window)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Recover".into(),
                "Cancel".into(),
            ))
            .blocking_show())
    })
    .await?;
    if confirmed {
        permit.with(|recovery| recovery.grant(Instant::now()));
        log::info!("recovery of the operator account was confirmed at this computer");
    }
    Ok(confirmed)
}

/// Saves a copy of the hub's key at a place the operator chooses in a native save dialog.
/// `false` when the operator cancelled. The key never passes through the web view.
#[tauri::command]
pub async fn local_hub_key_save<R: Runtime>(
    app: AppHandle<R>,
    webview: Webview<R>,
) -> Result<bool, LocalHubError> {
    from_main(webview.label())?;
    let key = crate::user_folders::of_this_user()
        .map(|application| crate::user_folders::hub_root(&application).join("vault.key"))
        .ok_or(LocalHubError::NoHome)?;
    if !key.is_file() {
        return Err(LocalHubError::NoKey);
    }
    let window = webview.window();
    let result = blocking(move || {
        let chosen = app
            .dialog()
            .file()
            .set_file_name(KEY_FILE_NAME)
            .set_parent(&window)
            .blocking_save_file();
        let Some(chosen) = chosen else {
            return Ok(false);
        };
        let destination = chosen
            .as_path()
            .ok_or(LocalHubError::Destination)?
            .to_path_buf();
        key_copy::copy(&key, &destination).map(|()| true)
    })
    .await;
    match &result {
        Ok(true) => log::info!("a copy of the hub's key was saved"),
        Ok(false) => {}
        Err(error) => log::warn!("the hub's key was not saved: {}", error.code()),
    }
    result
}
