//! Opens the page of the account sign-in in the browser of the phone (mobile plan M3-4, HMD-032). The
//! command has the name of the desktop one, `system_browser_open`, so the client calls one command on
//! each platform. Only `https` addresses; the URL is never logged (LOG-007).

use serde::{Serialize, Serializer, ser::SerializeStruct};

/// Why the address did not open. Carries no URL and no text of the operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PhoneBrowserError {
    /// The address is not a plain `https` address.
    #[error("the address cannot be opened")]
    Address,
    /// A web view other than the main one asked.
    #[cfg_attr(
        not(mobile),
        expect(dead_code, reason = "only the command of a phone makes it")
    )]
    #[error("only the main window can do this")]
    Caller,
    /// The operating system did not open it.
    #[error("the browser did not open")]
    Failed,
}

impl PhoneBrowserError {
    /// Stable code the client switches on.
    fn code(self) -> &'static str {
        match self {
            Self::Address => "url",
            Self::Caller => "caller",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code": ... }` and nothing else (WP-31 "Shell interface").
impl Serialize for PhoneBrowserError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("PhoneBrowserError", 1)?;
        out.serialize_field("code", self.code())?;
        out.end()
    }
}

/// `url` when it is an `https` address with a host, no user name or password in front of the host,
/// and no space or control character. The operating system then chooses the browser; it must never
/// get an address of another scheme, which could open another app.
pub(crate) fn checked(url: &str) -> Result<&str, PhoneBrowserError> {
    let rest = url
        .strip_prefix("https://")
        .ok_or(PhoneBrowserError::Address)?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let plain = !url
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\');
    if host.is_empty() || host.contains('@') || !plain {
        return Err(PhoneBrowserError::Address);
    }
    Ok(url)
}

/// Opens an `https` address in the default browser of the phone.
#[cfg(mobile)]
#[tauri::command]
pub async fn system_browser_open(
    webview: tauri::Webview,
    url: String,
) -> Result<(), PhoneBrowserError> {
    use tauri_plugin_opener::OpenerExt;
    if webview.label() != "main" {
        return Err(PhoneBrowserError::Caller);
    }
    let url = checked(&url)?;
    webview
        .opener()
        .open_url(url, None::<&str>)
        .map_err(|_| PhoneBrowserError::Failed)
}

#[cfg(test)]
mod tests {
    use super::{PhoneBrowserError, checked};

    #[test]
    fn m3_4_an_https_address_with_a_host_opens() {
        let page = "https://auth.example.com/device?user_code=ABCD-EFGH";
        assert_eq!(checked(page), Ok(page));
    }

    #[test]
    fn m3_4_any_other_address_is_refused() {
        for url in [
            "http://auth.example.com/device",
            "tel:+15550100",
            "anima://open",
            "https://",
            "https:///device",
            "https://user:secret@auth.example.com/",
            "https://auth.example.com/a b",
            "https://auth.example.com/\n",
            "https://auth.example.com\\@other.example",
            " https://auth.example.com/",
            "",
        ] {
            assert_eq!(checked(url), Err(PhoneBrowserError::Address), "{url:?}");
        }
    }

    #[test]
    fn log_007_the_error_crosses_to_the_client_as_a_code_only() {
        let json = serde_json::to_value(PhoneBrowserError::Failed).ok();
        assert_eq!(json, Some(serde_json::json!({ "code": "failed" })));
    }
}
