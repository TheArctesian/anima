use tauri::Webview;
use zeroize::Zeroizing;

use super::tokens::store;
use crate::cloud::CloudError;

const SCOPE: &str = "ing.anima.mobile.account";

pub(crate) async fn read(
    webview: &Webview,
    client_id: &str,
) -> Result<Option<Zeroizing<String>>, CloudError> {
    store(webview)
        .read(SCOPE, client_id)
        .await
        .map(|token| token.map(Zeroizing::new))
        .map_err(|_| CloudError::Keystore)
}

pub(crate) async fn write(
    webview: &Webview,
    client_id: &str,
    token: &Zeroizing<String>,
) -> Result<(), CloudError> {
    store(webview)
        .write(SCOPE, client_id, token)
        .await
        .map_err(|_| CloudError::Keystore)
}

pub(crate) async fn clear(webview: &Webview, client_id: &str) -> Result<(), CloudError> {
    store(webview)
        .clear(SCOPE, client_id)
        .await
        .map_err(|_| CloudError::Keystore)
}
