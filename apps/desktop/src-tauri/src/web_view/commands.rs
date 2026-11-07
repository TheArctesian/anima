//! The web view commands of the shell interface (WP-31), for the main web view only: the
//! capability `desktop` allows them for `main`, and each command checks its caller too.
//! Every failure is logged as its code only (LOG-007).

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, Webview, ipc::InvokeBody, ipc::Request};
use tauri_plugin_log::log;

use super::{
    Fingerprint, PinnedOrigin, Tabs, WebViewError, bounds::Bounds, event::MAIN, history::Step,
    leaf_fingerprint, opening::Opening, pinning_available, tabs::TabId, view,
};

/// How long reading a leaf certificate may take: name lookup, connect and handshake.
const CERTIFICATE_PATIENCE: Duration = Duration::from_secs(10);

/// Refuses every caller but the main web view. Device web views have no capability, so the
/// ACL already refuses them; this is the second wall.
pub fn caller(label: &str) -> Result<(), WebViewError> {
    if label == MAIN {
        Ok(())
    } else {
        Err(WebViewError::Caller)
    }
}

fn reported<T>(result: Result<T, WebViewError>) -> Result<T, WebViewError> {
    if let Err(error) = &result {
        log::warn!("a web view command was refused: {}", error.code());
    }
    result
}

fn native(_: tauri::Error) -> WebViewError {
    WebViewError::Native
}

/// The checked tab ID and its open web view.
fn open_view(app: &AppHandle, tab_id: &str) -> Result<(TabId, Webview), WebViewError> {
    let id = TabId::parse(tab_id)?;
    let view = app
        .get_webview(&id.view_label())
        .ok_or(WebViewError::UnknownTab)?;
    Ok((id, view))
}

/// Whether device web views can pin a certificate here (D-6).
#[tauri::command]
pub fn web_view_pinning() -> bool {
    pinning_available()
}

/// Opens a device web view for a tab, in the main window at `bounds`, with the device's
/// own store, then loads `url`. `https` is refused (`pinning_unavailable`) where the
/// platform cannot pin; the client then uses the system browser.
#[tauri::command]
pub async fn web_view_open(webview: Webview, request: Request<'_>) -> Result<(), WebViewError> {
    let body = match request.body() {
        InvokeBody::Json(body) => Some(body.clone()),
        InvokeBody::Raw(_) => None,
    };
    reported(open(&webview, body).await)
}

async fn open(caller_view: &Webview, body: Option<serde_json::Value>) -> Result<(), WebViewError> {
    caller(caller_view.label())?;
    open_tab(
        caller_view.app_handle(),
        &body.ok_or(WebViewError::Arguments)?,
    )
    .await
}

/// Opens a tab's device web view from `web_view_open`'s arguments, once the caller is
/// checked: the view on its blank start, then the trust hook, then the first real load.
pub async fn open_tab(app: &AppHandle, body: &serde_json::Value) -> Result<(), WebViewError> {
    let new = Opening::read(body)?.check(pinning_available())?;
    let window = app.get_window(MAIN).ok_or(WebViewError::Window)?;
    let url = new.tab.url.as_url().clone();
    let generation = app.state::<Tabs>().reserve(new.id.clone(), new.tab)?;
    let (id, device, rect) = (new.id.clone(), new.device, new.rect);
    let attached =
        tauri::async_runtime::spawn_blocking(move || view::attach(&window, &id, device, rect))
            .await
            .map_err(|_| WebViewError::Native)
            .flatten();
    let view = match attached {
        Ok(view) => view,
        // This open made no view; one under the label, if any, belongs to another open.
        Err(error) => {
            app.state::<Tabs>().release(&new.id, generation);
            return Err(error);
        }
    };
    // The trust hook goes on the blank view before the first real load, or nothing loads.
    // A tab closed, or closed and opened again, while this open waited gets no load either.
    let loaded = view::guard(&view, &new.id).await.and_then(|()| {
        if app.state::<Tabs>().holds(&new.id, generation) {
            view.navigate(url).map_err(native)
        } else {
            Err(WebViewError::UnknownTab)
        }
    });
    if let Err(error) = loaded {
        app.state::<Tabs>().release(&new.id, generation);
        return view.close().map_err(native).and(Err(error));
    }
    Ok(())
}

/// Moves and sizes a tab's web view in the main window. A popped-out view fills its own
/// window, so this leaves it alone.
#[tauri::command]
pub async fn web_view_place(
    webview: Webview,
    tab_id: &str,
    bounds: Bounds,
) -> Result<(), WebViewError> {
    reported(caller(webview.label()).and_then(|()| {
        let (id, view) = open_view(webview.app_handle(), tab_id)?;
        let rect = bounds.rect()?;
        if webview.state::<Tabs>().get(&id)?.popped_out {
            return Ok(());
        }
        view.set_bounds(rect).map_err(native)
    }))
}

/// Shows or hides a tab's web view in the main window (the tab is hidden, or an overlay is
/// open). A popped-out view stays shown in its own window.
#[tauri::command]
pub async fn web_view_show(
    webview: Webview,
    tab_id: &str,
    shown: bool,
) -> Result<(), WebViewError> {
    reported(caller(webview.label()).and_then(|()| {
        let (id, view) = open_view(webview.app_handle(), tab_id)?;
        if webview.state::<Tabs>().get(&id)?.popped_out {
            return Ok(());
        }
        if shown { view.show() } else { view.hide() }.map_err(native)
    }))
}

/// Takes a history step in a tab's web view, or reloads it.
#[tauri::command]
pub async fn web_view_go(webview: Webview, tab_id: &str, to: &str) -> Result<(), WebViewError> {
    reported(caller(webview.label()).and_then(|()| {
        let (_, view) = open_view(webview.app_handle(), tab_id)?;
        match Step::parse(to)?.script() {
            Some(script) => view.eval(script),
            None => view.reload(),
        }
        .map_err(native)
    }))
}

/// Pins the certificate the operator accepted for the tab's `https` origin, then loads the
/// tab's address again so the new pin decides.
#[tauri::command]
pub async fn web_view_pin(webview: Webview, tab_id: &str, pin: &str) -> Result<(), WebViewError> {
    reported(caller(webview.label()).and_then(|()| pin_tab(webview.app_handle(), tab_id, pin)))
}

/// Pins `pin` to a tab's `https` origin and loads the tab's address again, once the caller is
/// checked.
pub fn pin_tab(app: &AppHandle, tab_id: &str, pin: &str) -> Result<(), WebViewError> {
    let (id, view) = open_view(app, tab_id)?;
    let fingerprint = pin.parse::<Fingerprint>()?;
    let tab = app.state::<Tabs>().set_pin(&id, fingerprint)?;
    view.navigate(tab.url.as_url().clone()).map_err(native)
}

/// Moves a tab's web view to a window of its own, with the same store. Again: no change.
#[tauri::command]
pub async fn web_view_pop_out(webview: Webview, tab_id: String) -> Result<(), WebViewError> {
    reported(caller(webview.label()).and_then(|()| {
        let (id, view) = open_view(webview.app_handle(), &tab_id)?;
        match webview.state::<Tabs>().pop_out(&id)? {
            Some(generation) => view::pop_out(webview.app_handle(), &view, &id, generation),
            None => Ok(()),
        }
    }))
}

/// Closes a tab's web view. Closing a tab that is not open is not an error.
#[tauri::command]
pub async fn web_view_close(webview: Webview, tab_id: &str) -> Result<(), WebViewError> {
    reported(caller(webview.label()).and_then(|()| {
        let id = TabId::parse(tab_id)?;
        let popped_out = webview
            .state::<Tabs>()
            .remove(&id)
            .is_some_and(|tab| tab.popped_out);
        view::close(webview.app_handle(), &id, popped_out)
    }))
}

/// The answer of `web_certificate_read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeafCertificate {
    pub fingerprint_sha256: Fingerprint,
}

/// The SHA-256 fingerprint of the leaf certificate an `https` address presents now, for the
/// system-browser mode (D-9).
#[tauri::command]
pub async fn web_certificate_read(
    webview: Webview,
    url: String,
) -> Result<LeafCertificate, WebViewError> {
    reported(certificate(webview.label(), &url).await)
}

async fn certificate(label: &str, url: &str) -> Result<LeafCertificate, WebViewError> {
    caller(label)?;
    let origin = PinnedOrigin::parse(url)?;
    let fingerprint_sha256 = leaf_fingerprint(&origin, CERTIFICATE_PATIENCE)
        .await
        .map_err(WebViewError::Certificate)?;
    Ok(LeafCertificate { fingerprint_sha256 })
}
