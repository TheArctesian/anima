//! M-1 (WEB-004): a closed device web view is freed. The proof holds the `WKWebView` through
//! a weak reference on the main thread, closes the tab and checks the view is gone, so no
//! retain cycle runs through the trust hook's proxy, its decision or wry's delegate.

use std::cell::RefCell;

use objc2::rc::Weak;
use objc2_web_kit::WKWebView;
use tauri::{AppHandle, Webview};
use tokio::{sync::oneshot, time::sleep};

use crate::{
    proof::{Proof, SETTLE, Tab},
    web_view::{WebViewError, commands::close_tab, macos_trust},
};

thread_local! {
    /// The `WKWebView` the proof watches, held weakly, on the main thread.
    static WATCHED: RefCell<Option<Weak<WKWebView>>> = const { RefCell::new(None) };
}

/// Starts watching `view`'s `WKWebView` through a weak reference.
async fn watch(view: &Webview) -> Result<(), String> {
    let (sent, done) = oneshot::channel();
    view.with_webview(move |platform| {
        let watched = macos_trust::web_view(&platform).map(|view| {
            WATCHED.with_borrow_mut(|watched| *watched = Some(Weak::new(view)));
        });
        // The proof waits for this answer; it is gone only if the proof has already failed.
        let _proof_gone = sent.send(watched.map_err(WebViewError::code));
    })
    .map_err(|error| error.to_string())?;
    done.await
        .map_err(|_| "no watch".to_owned())?
        .map_err(str::to_owned)
}

/// Whether the watched `WKWebView` is still alive.
async fn watched_alive(app: &AppHandle) -> Result<bool, String> {
    let (sent, done) = oneshot::channel();
    app.run_on_main_thread(move || {
        let alive = WATCHED
            .with_borrow(|watched| watched.as_ref().is_some_and(|view| view.load().is_some()));
        let _proof_gone = sent.send(alive);
    })
    .map_err(|error| error.to_string())?;
    done.await.map_err(|_| "no answer".to_owned())
}

/// M-1: closing the tab frees its `WKWebView`, so its page stops: no retain cycle runs
/// through the proxy, its decision closure or wry's delegate.
pub async fn closed_tab_freed(tab: &Tab, proof: &mut Proof) -> Result<(), String> {
    watch(&tab.view()?).await?;
    let before = watched_alive(&tab.app).await?;
    close_tab(&tab.app, tab.id).map_err(|error| error.code().to_owned())?;
    sleep(SETTLE * 2).await;
    let after = watched_alive(&tab.app).await?;
    proof.check(
        "closed-tab-freed",
        before && !after,
        &format!("WKWebView alive before close {before}; 2 s after close {after}"),
    );
    Ok(())
}
