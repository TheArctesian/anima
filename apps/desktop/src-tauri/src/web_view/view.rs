//! The native device web view (D-2, D-3, WEB-006, WEB-007): built as a child of the main
//! window on a blank start, with the device's store, the `http`/`https` filter, new windows
//! kept in the same view, page loads reported to the main web view and, on macOS, the trust
//! hook that answers certificate challenges with the tab's pin. OS work only; the rules it
//! applies live in the pure units beside it.

use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Rect, Url, Webview, WebviewBuilder,
    WebviewUrl, Window, WindowEvent,
    webview::{NewWindowResponse, PageLoadEvent},
    window::WindowBuilder,
};

#[cfg(target_os = "macos")]
use super::trust::{Answer, Presented, rule};
use super::{
    Tabs, WebViewError,
    address::is_web,
    event::{self, Failure, WebViewEvent},
    storage::DeviceId,
    tabs::TabId,
};

/// The address a device web view is created on. The real address is loaded only after the
/// view exists, so a trust hook installed on the new view sees the first real load (OD-7).
/// The filter refuses it like any non-web scheme, which leaves the view empty until then.
const BLANK: &str = "about:blank";

/// How long the main thread may take to put the trust hook on a new view.
#[cfg(target_os = "macos")]
const GUARD_PATIENCE: std::time::Duration = std::time::Duration::from_secs(10);

/// The title of a popped-out web view's window: fixed text, never the page's (LOG-007).
const POP_OUT_TITLE: &str = "Anima";
const POP_OUT_WIDTH: f64 = 1200.0;
const POP_OUT_HEIGHT: f64 = 800.0;

fn native(_: tauri::Error) -> WebViewError {
    WebViewError::Native
}

/// Creates the device web view for `id` in `window` at `rect`, on the blank start.
pub fn attach(
    window: &Window,
    id: &TabId,
    device: DeviceId,
    rect: Rect,
) -> Result<Webview, WebViewError> {
    let builder = with_store(
        builder(window.app_handle(), id)?,
        window.app_handle(),
        device,
    )?;
    window
        .add_child(builder, rect.position, rect.size)
        .map_err(native)
}

fn builder(app: &AppHandle, id: &TabId) -> Result<WebviewBuilder<tauri::Wry>, WebViewError> {
    let blank = Url::parse(BLANK).map_err(|_| WebViewError::Native)?;
    let (app, label, loads) = (app.clone(), id.view_label(), id.clone());
    let opener = id.clone();
    Ok(
        WebviewBuilder::new(id.view_label(), WebviewUrl::External(blank))
            .focused(false)
            // Files dropped on the page reach the page, not the shell's drop events.
            .disable_drag_drop_handler()
            .on_navigation(is_web)
            .on_new_window(move |url, _features| {
                if is_web(&url) {
                    follow(app.clone(), label.clone(), opener.clone(), url);
                }
                NewWindowResponse::Deny
            })
            .on_page_load(move |view, payload| {
                let finished = payload.event() == PageLoadEvent::Finished;
                if let Some(event) = WebViewEvent::page(&loads, payload.url(), finished) {
                    event::send(&view, &event);
                }
            }),
    )
}

/// Puts the trust hook on a new device web view and waits until it is there (WEB-008, OD-7).
/// Call it before the first real load; on an error, load nothing and close the view.
#[cfg(target_os = "macos")]
pub async fn guard(view: &Webview, id: &TabId) -> Result<(), WebViewError> {
    let (app, tab) = (view.app_handle().clone(), id.clone());
    let (sent, installed) = tokio::sync::oneshot::channel();
    view.with_webview(move |platform| {
        let decide = Box::new(
            move |presented: &Presented<'_>, system: &dyn Fn() -> bool| {
                decide(&app, &tab, presented, system)
            },
        );
        // An error here means the opener stopped waiting; it closes the view itself.
        let _opener_gone = sent.send(super::macos_trust::install(&platform, decide));
    })
    .map_err(native)?;
    tokio::time::timeout(GUARD_PATIENCE, installed)
        .await
        .map_err(|_| WebViewError::Native)?
        .map_err(|_| WebViewError::Native)?
}

/// No trust hook outside macOS yet: `https` opens in the system browser (D-6).
#[cfg(not(target_os = "macos"))]
#[expect(
    clippy::unused_async,
    reason = "the macOS twin waits for the main thread; both answer the same type"
)]
pub async fn guard(_view: &Webview, _id: &TabId) -> Result<(), WebViewError> {
    Ok(())
}

/// Answers one challenge in tab `id` from the tab's pin, and tells the main web view what the
/// operator must see. On the main thread, inside `WebKit`'s callback: the tab lock is held only
/// to copy the tab, and the event is sent from a task. A closed tab loads nothing.
#[cfg(target_os = "macos")]
fn decide(
    app: &AppHandle,
    id: &TabId,
    presented: &Presented<'_>,
    system_trust: &dyn Fn() -> bool,
) -> Answer {
    let Some(tab) = app.try_state::<Tabs>().and_then(|tabs| tabs.get(id).ok()) else {
        return Answer::Cancel;
    };
    let ruling = rule(
        tab.pin.as_ref(),
        tab.origin.as_ref(),
        presented,
        system_trust,
    );
    if let Some(event) = ruling
        .shown
        .and_then(|shown| WebViewEvent::refusal(id, shown))
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { event::send(&app, &event) });
    }
    ruling.answer
}

/// Loads a page's new-window request in its own web view instead. Off the `WebKit` callback,
/// so the view is not driven from inside its own delegate; the filter checks it again.
fn follow(app: AppHandle, label: String, id: TabId, url: Url) {
    tauri::async_runtime::spawn(async move {
        let Some(view) = app.get_webview(&label) else {
            return;
        };
        if view.navigate(url).is_err() {
            let failed = WebViewEvent::Failed {
                tab_id: id.as_str().to_owned(),
                reason: Failure::Navigate,
            };
            event::send(&app, &failed);
        }
    });
}

/// macOS: the `WKWebsiteDataStore` named by the device's UUID (macOS 14+).
#[cfg(target_os = "macos")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Linux twin can fail; both answer the same type"
)]
fn with_store(
    builder: WebviewBuilder<tauri::Wry>,
    _app: &AppHandle,
    device: DeviceId,
) -> Result<WebviewBuilder<tauri::Wry>, WebViewError> {
    Ok(builder.data_store_identifier(device.store_identifier()))
}

/// Linux: a WebKitGTK data folder per device under the app's data folder.
#[cfg(not(target_os = "macos"))]
fn with_store(
    builder: WebviewBuilder<tauri::Wry>,
    app: &AppHandle,
    device: DeviceId,
) -> Result<WebviewBuilder<tauri::Wry>, WebViewError> {
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|_| WebViewError::Store)?
        .join(super::storage::STORE_FOLDER)
        .join(device.folder_name());
    Ok(builder.data_directory(folder))
}

/// Moves `view` to a window of its own, filling it; the store goes with the view. Closing
/// that window closes the web view and ends the tab.
pub fn pop_out(app: &AppHandle, view: &Webview, id: &TabId) -> Result<(), WebViewError> {
    let window = WindowBuilder::new(app, id.window_label())
        .title(POP_OUT_TITLE)
        .inner_size(POP_OUT_WIDTH, POP_OUT_HEIGHT)
        .build()
        .map_err(native)?;
    let (ended, owner) = (app.clone(), id.clone());
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            ended.state::<Tabs>().remove(&owner);
        }
    });
    view.reparent(&window).map_err(native)?;
    view.set_bounds(Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: LogicalSize::new(POP_OUT_WIDTH, POP_OUT_HEIGHT).into(),
    })
    .map_err(native)?;
    view.set_auto_resize(true).map_err(native)?;
    view.show().map_err(native)
}

/// Closes the tab's web view, and its own window if it was popped out.
pub fn close(app: &AppHandle, id: &TabId, popped_out: bool) -> Result<(), WebViewError> {
    if popped_out && let Some(window) = app.get_window(&id.window_label()) {
        return window.close().map_err(native);
    }
    match app.get_webview(&id.view_label()) {
        Some(view) => view.close().map_err(native),
        None => Ok(()),
    }
}
