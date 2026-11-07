//! The `web-view` event: what a device web view reports, sent to the main web view only
//! (WP-31 "Shell interface"). The URL goes to the main web view and nowhere else; it is
//! never logged (LOG-007).

use serde::Serialize;
use tauri::{Emitter, EventTarget, Runtime, Url};
use tauri_plugin_log::log;

use super::{Fingerprint, Verdict, address::is_web, tabs::TabId};

/// The event's name.
pub const EVENT: &str = "web-view";

/// The label of the main window and its web view, the only place the event goes.
pub const MAIN: &str = "main";

/// Why a web view could not show a page: a short code, never a URL or page text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// The web view did not take an address the page asked for in a new window.
    Navigate,
}

/// One `web-view` event payload: `{ tabId, kind, ... }`, exactly the client's fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum WebViewEvent {
    Loading {
        tab_id: String,
        url: String,
    },
    Loaded {
        tab_id: String,
        url: String,
    },
    #[cfg_attr(
        not(any(target_os = "macos", test)),
        expect(
            dead_code,
            reason = "only macOS has a trust hook until OD-7 runs on Linux"
        )
    )]
    Untrusted {
        tab_id: String,
        fingerprint_sha256: Fingerprint,
    },
    #[cfg_attr(
        not(any(target_os = "macos", test)),
        expect(
            dead_code,
            reason = "only macOS has a trust hook until OD-7 runs on Linux"
        )
    )]
    Mismatch {
        tab_id: String,
        expected: Fingerprint,
        presented: Fingerprint,
    },
    Failed {
        tab_id: String,
        reason: Failure,
    },
}

impl WebViewEvent {
    /// `LOADING` or `LOADED` for a page on `url`, or `None` if the address is not one a
    /// device web view may show (the blank page it opens on, a refused scheme).
    pub fn page(tab: &TabId, url: &Url, finished: bool) -> Option<Self> {
        if !is_web(url) {
            return None;
        }
        let (tab_id, url) = (tab.as_str().to_owned(), url.to_string());
        Some(if finished {
            Self::Loaded { tab_id, url }
        } else {
            Self::Loading { tab_id, url }
        })
    }

    /// `UNTRUSTED` or `MISMATCH` for a verdict the operator must see, or `None` for one the
    /// web view acts on alone.
    #[cfg_attr(
        not(any(target_os = "macos", test)),
        expect(
            dead_code,
            reason = "only macOS has a trust hook until OD-7 runs on Linux"
        )
    )]
    pub fn refusal(tab: &TabId, verdict: Verdict) -> Option<Self> {
        let tab_id = tab.as_str().to_owned();
        match verdict {
            Verdict::Untrusted(fingerprint_sha256) => Some(Self::Untrusted {
                tab_id,
                fingerprint_sha256,
            }),
            Verdict::Mismatch {
                expected,
                presented,
            } => Some(Self::Mismatch {
                tab_id,
                expected,
                presented,
            }),
            Verdict::Accept | Verdict::Default => None,
        }
    }
}

/// Sends `event` to the main web view only. The client listens with the target
/// `WebviewWindow` `main`, which a `Webview` target would not reach. Device web views cannot
/// listen: listening is IPC, and no capability names them (D-3).
pub fn send<R: Runtime>(emitter: &impl Emitter<R>, event: &WebViewEvent) {
    if emitter
        .emit_to(EventTarget::webview_window(MAIN), EVENT, event)
        .is_err()
    {
        log::warn!("a web-view event was not delivered");
    }
}
