//! The desktop process log (LOG-008): the log plugin writes to the OS log folder, and one
//! command shows that folder in the file manager. The web view gives the command no path.
//! Nothing here logs a token, a password or session content (LOG-007).

mod error;
mod folder;
mod line;

#[cfg(test)]
mod tests;

pub use error::ProcessLogError;

use tauri::{AppHandle, Manager, Runtime, plugin::TauriPlugin};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy, log};

/// A file rotates at 1 MiB and five old files are kept, so the folder stays near 6 MiB.
const MAX_FILE_BYTES: u128 = 1024 * 1024;
const KEPT_FILES: usize = 5;

/// The log plugin: `info` and above to the OS log folder, and to stdout in debug builds only.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let mut targets = vec![Target::new(TargetKind::LogDir { file_name: None })];
    if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stdout));
    }
    tauri_plugin_log::Builder::new()
        .targets(targets)
        .level(log::LevelFilter::Info)
        .max_file_size(MAX_FILE_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(KEPT_FILES))
        .timezone_strategy(TimezoneStrategy::UseUtc)
        .format(|out, message, record| {
            let now = TimezoneStrategy::UseUtc.get_now();
            let stamp = line::Stamp {
                date: (now.year(), u8::from(now.month()), now.day()),
                time: (now.hour(), now.minute(), now.second()),
            };
            let message = message.to_string();
            out.finish(format_args!(
                "{}",
                line::render(&stamp, record.level(), record.target(), &message)
            ));
        })
        .build()
}

/// Shows this app's log folder in the OS file manager, creating it first if it is missing.
#[tauri::command]
pub async fn process_log_folder_open<R: Runtime>(app: AppHandle<R>) -> Result<(), ProcessLogError> {
    let folder = app
        .path()
        .app_log_dir()
        .map_err(|_| ProcessLogError::Unavailable)?;
    let opened =
        tauri::async_runtime::spawn_blocking(move || folder::open_with(&folder, folder::OPENER))
            .await
            .map_err(|_| ProcessLogError::Failed)
            .flatten();
    if let Err(error) = opened {
        log::warn!("the log folder did not open: {}", error.code());
    }
    opened
}
