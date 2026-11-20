//! Hub tokens and cloud refresh tokens share a native store with separate entry scopes.

use tauri::{Manager, State, Url, Webview, Wry};
use tauri_plugin_account_session::SecureTokens;
use zeroize::Zeroizing;

const HUB_SCOPE: &str = "ing.anima.mobile.hub";

#[derive(serde::Serialize)]
pub struct TokenProblem {
    code: &'static str,
}

pub(crate) fn store(webview: &Webview) -> State<'_, SecureTokens<Wry>> {
    webview.state()
}

fn account_for(webview: &Webview, origin: &str) -> Result<String, TokenProblem> {
    if webview.label() != "main" {
        return Err(TokenProblem { code: "caller" });
    }
    let canonical = Url::parse(origin).is_ok_and(|url| {
        (url.scheme() == "https" || (url.scheme() == "http" && url.host_str() == Some("127.0.0.1")))
            && url.origin().ascii_serialization() == origin
    });
    if !canonical {
        return Err(TokenProblem { code: "origin" });
    }
    if origin.starts_with("http://") {
        return crate::phone_link::listener_key(origin)
            .map(|key| format!("link:{key}"))
            .ok_or(TokenProblem {
                code: "unavailable",
            });
    }
    Ok(origin.to_owned())
}

#[tauri::command]
pub async fn access_token_read(
    webview: Webview,
    origin: String,
) -> Result<Option<String>, TokenProblem> {
    let account = account_for(&webview, &origin)?;
    store(&webview)
        .read(HUB_SCOPE, &account)
        .await
        .map_err(|_| TokenProblem { code: "failed" })
}

#[tauri::command]
pub async fn access_token_write(
    webview: Webview,
    origin: String,
    token: String,
) -> Result<(), TokenProblem> {
    let token = Zeroizing::new(token);
    let account = account_for(&webview, &origin)?;
    if token.is_empty() {
        return Err(TokenProblem { code: "empty" });
    }
    store(&webview)
        .write(HUB_SCOPE, &account, &token)
        .await
        .map_err(|_| TokenProblem { code: "failed" })
}

#[tauri::command]
pub async fn access_token_clear(webview: Webview, origin: String) -> Result<(), TokenProblem> {
    let account = account_for(&webview, &origin)?;
    store(&webview)
        .clear(HUB_SCOPE, &account)
        .await
        .map_err(|_| TokenProblem { code: "failed" })
}
