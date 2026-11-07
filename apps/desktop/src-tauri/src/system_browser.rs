//! Opens a device's web interface in the operator's own browser (D-9): where a device web
//! view cannot pin its certificate, the client shows the fingerprint and sends the operator
//! here. Only `http` and `https` addresses; the URL is never logged (LOG-007).

use std::{
    io::ErrorKind,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Serialize, Serializer, ser::SerializeStruct};
use tauri_plugin_log::log;

use crate::web_view::{WebUrl, WebViewError};

/// The URL opener: `open` on macOS, `xdg-open` on Linux. An absolute path, so `PATH`
/// cannot swap it.
const OPENER: &str = if cfg!(target_os = "macos") {
    "/usr/bin/open"
} else {
    "/usr/bin/xdg-open"
};

/// How long to wait for the opener's exit status. `open` and most `xdg-open` back ends hand
/// the address to the browser and exit at once; some `xdg-open` fallbacks run the browser in
/// the foreground and exit only when it closes.
const PATIENCE: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(20);

/// Why the address did not open in the system browser. Carries no URL or OS error text.
///
/// - `url`, `scheme`, `userinfo`, `host`: the address broke that rule.
/// - `unavailable`: there is no opener on this system.
/// - `failed`: the opener ran and reported a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SystemBrowserError {
    #[error("the address cannot be opened: {}", .0.code())]
    Address(WebViewError),
    #[error("there is no browser opener on this system")]
    Unavailable,
    #[error("the browser did not open")]
    Failed,
}

impl SystemBrowserError {
    /// Stable code the client switches on.
    fn code(self) -> &'static str {
        match self {
            Self::Address(error) => error.code(),
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code": ... }` and nothing else (WP-31 "Shell interface").
impl Serialize for SystemBrowserError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("SystemBrowserError", 1)?;
        out.serialize_field("code", self.code())?;
        out.end()
    }
}

/// Runs `opener` with `url` as its one argument (no shell). A checked URL starts with its
/// scheme, so the opener cannot read it as an option.
///
/// An exit within `patience` decides the result. An opener still running after `patience` is
/// taken as a browser in the foreground: the call succeeds and a thread reaps the child when
/// it exits, so a blocking `xdg-open` never leaves the command pending.
fn open_with(opener: &str, url: &WebUrl, patience: Duration) -> Result<(), SystemBrowserError> {
    let mut child = Command::new(opener)
        .arg(url.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => SystemBrowserError::Unavailable,
            _ => SystemBrowserError::Failed,
        })?;
    let deadline = Instant::now() + patience;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) | Err(_) => return Err(SystemBrowserError::Failed),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
            Ok(None) => {
                // The exit status no longer matters; waiting only reaps the child.
                thread::spawn(move || child.wait());
                return Ok(());
            }
        }
    }
}

/// Opens an `http` or `https` address in the system's default browser.
#[tauri::command]
pub async fn system_browser_open(
    webview: tauri::Webview,
    url: String,
) -> Result<(), SystemBrowserError> {
    let opened = match crate::web_view::caller(webview.label()).and_then(|()| WebUrl::parse(&url)) {
        Ok(url) => tauri::async_runtime::spawn_blocking(move || open_with(OPENER, &url, PATIENCE))
            .await
            .map_err(|_| SystemBrowserError::Failed)
            .flatten(),
        Err(error) => Err(SystemBrowserError::Address(error)),
    };
    if let Err(error) = opened {
        log::warn!("the system browser did not open: {}", error.code());
    }
    opened
}

#[cfg(test)]
mod tests {
    //! The tests run `true`, `false` or a missing program in place of the opener, so no test
    //! opens a browser.

    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    use super::{PATIENCE, SystemBrowserError, open_with};
    use crate::web_view::{WebUrl, WebViewError};

    fn open(opener: &str) -> Result<(), SystemBrowserError> {
        open_with(opener, &url(), PATIENCE)
    }

    fn url() -> WebUrl {
        WebUrl::parse("https://192.0.2.7:8443/admin").unwrap()
    }

    #[test]
    fn web_004_an_opener_that_succeeds_opens_the_address() {
        assert_eq!(open("true"), Ok(()));
    }

    #[test]
    fn web_004_an_opener_that_fails_is_reported_as_failed() {
        assert_eq!(open("false"), Err(SystemBrowserError::Failed));
    }

    #[test]
    fn web_004_a_missing_opener_is_reported_as_unavailable() {
        assert_eq!(
            open("/nonexistent/anima-opener"),
            Err(SystemBrowserError::Unavailable)
        );
    }

    #[test]
    fn web_004_an_opener_still_running_after_patience_succeeds_without_waiting_for_it() {
        let folder = std::env::temp_dir().join(format!("anima-browser-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let opener = folder.join("browser in the foreground");
        fs::write(&opener, "#!/bin/sh\nexec sleep 10\n").unwrap();
        fs::set_permissions(&opener, fs::Permissions::from_mode(0o755)).unwrap();
        let started = Instant::now();
        let answer = open_with(opener.to_str().unwrap(), &url(), Duration::from_millis(100));
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(answer, Ok(()));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn log_007_the_error_crosses_to_the_client_as_a_code_only() {
        let json = serde_json::to_value(SystemBrowserError::Address(WebViewError::Scheme)).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "scheme" }));
        let json = serde_json::to_value(SystemBrowserError::Failed).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "failed" }));
    }
}
