//! Register the bundled Windows CLI in the current user's PATH (WIN2, INS-006).
//! The installer owns the executable; Remove unregisters its folder without deleting it.
use anima_windows::search_path;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{Runtime, Webview};

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct CommandLineError {
    code: &'static str,
    message: &'static str,
}

fn failure(code: &'static str) -> CommandLineError {
    let message = match code {
        "caller" => "only the main window may change the command line tool",
        "not-bundled" => "this build has no trusted bundled command line tool",
        "changed" => "the proposed folder changed; read it again before installing",
        _ => "the current user's PATH could not be read or changed",
    };
    CommandLineError { code, message }
}

#[derive(Serialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Status {
    Unavailable {
        reason: &'static str,
    },
    Installed {
        path: String,
    },
    NotInstalled {
        folder: String,
        path: String,
        on_path: bool,
        create: bool,
        profile_line: Option<String>,
    },
}

fn program() -> Result<PathBuf, CommandLineError> {
    if cfg!(debug_assertions) {
        return Err(failure("not-bundled"));
    }
    let executable = std::env::current_exe().map_err(|_| failure("not-bundled"))?;
    let program = executable
        .parent()
        .ok_or_else(|| failure("not-bundled"))?
        .join("anima.exe");
    anima_cli::mcp::install::setup::check_command(&program, 0)
        .map_err(|_| failure("not-bundled"))?;
    Ok(program)
}

fn status(program: &Path) -> Result<Status, CommandLineError> {
    let folder = program.parent().ok_or_else(|| failure("not-bundled"))?;
    let registered = search_path::registered(folder).map_err(|_| failure("failed"))?;
    let path = program.to_string_lossy().into_owned();
    Ok(if registered {
        Status::Installed { path }
    } else {
        Status::NotInstalled {
            folder: folder.to_string_lossy().into_owned(),
            path,
            on_path: false,
            create: false,
            profile_line: None,
        }
    })
}

pub(crate) struct Installed {
    pub anima: PathBuf,
    pub home: PathBuf,
    pub path: Vec<PathBuf>,
}

/// The installed bundled CLI and the user's paths for coding-agent setup.
pub(crate) async fn installed() -> Option<Installed> {
    tauri::async_runtime::spawn_blocking(installed_here)
        .await
        .ok()
        .flatten()
}

fn installed_here() -> Option<Installed> {
    let anima = program().ok()?;
    if !matches!(status(&anima).ok()?, Status::Installed { .. }) {
        return None;
    }
    let home = PathBuf::from(std::env::var_os("USERPROFILE")?);
    if !home.is_absolute() {
        return None;
    }
    let path =
        std::env::var_os("PATH").map_or_default(|path| std::env::split_paths(&path).collect());
    Some(Installed { anima, home, path })
}

fn from_main(label: &str) -> Result<(), CommandLineError> {
    crate::web_view::caller(label).map_err(|_| failure("caller"))
}

#[tauri::command]
pub async fn command_line_read<R: Runtime>(
    webview: Webview<R>,
) -> Result<Status, CommandLineError> {
    from_main(webview.label())?;
    Ok(program()
        .and_then(|program| status(&program))
        .unwrap_or_else(|error| Status::Unavailable { reason: error.code }))
}

async fn change(folder: Option<String>) -> Result<Status, CommandLineError> {
    tauri::async_runtime::spawn_blocking(move || {
        let program = program()?;
        let actual = program.parent().ok_or_else(|| failure("not-bundled"))?;
        if folder
            .as_ref()
            .is_some_and(|folder| Path::new(folder) != actual)
        {
            return Err(failure("changed"));
        }
        search_path::register(actual, folder.is_some()).map_err(|_| failure("failed"))?;
        status(&program)
    })
    .await
    .map_err(|_| failure("failed"))?
}

#[tauri::command]
pub async fn command_line_install<R: Runtime>(
    webview: Webview<R>,
    folder: String,
) -> Result<Status, CommandLineError> {
    from_main(webview.label())?;
    change(Some(folder)).await
}

#[tauri::command]
pub async fn command_line_remove<R: Runtime>(
    webview: Webview<R>,
) -> Result<Status, CommandLineError> {
    from_main(webview.label())?;
    change(None).await
}
