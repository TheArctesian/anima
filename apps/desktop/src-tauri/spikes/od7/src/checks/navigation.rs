//! Check 4 (WEB-007, D-3): a device web view goes only to `http` and `https`. A hostile page tries
//! each target itself; then Rust asks for each target, to prove the filter (not just WebKit)
//! refuses it. Finally the page goes to an http URL, which must be allowed.

use std::time::Duration;

use tauri::{Url, Webview};

use super::device::{self, STORE_NAV};
use super::{Harness, Results};
use crate::event::{Event, Journal, is_report};
use crate::pages::NAVIGATION_TARGETS;

const LABEL: &str = "web-nav";

pub async fn check(harness: &Harness, journal: &mut Journal, results: &mut Results) {
    let from = journal.mark();
    let view = match device::open(harness, LABEL, Some(STORE_NAV), harness.http_url("/nav")) {
        Ok(view) => view,
        Err(reason) => return results.record(false, "nav-setup", reason),
    };
    let landed = journal
        .until(
            from,
            Duration::from_secs(12),
            |e| matches!(e, Event::Served { path, .. } if path == "/nav/landed"),
        )
        .await;
    for (index, target) in NAVIGATION_TARGETS.iter().enumerate() {
        page_attempt(journal, results, from, (index, target));
    }
    results.record(
        landed.is_some(),
        "nav-http-allowed",
        "after the refused targets the page navigated to http://127.0.0.1/nav/landed",
    );
    for target in NAVIGATION_TARGETS {
        rust_attempt(&view, journal, results, target).await;
    }
}

/// The page is still alive after trying `target` if it reported index `index` from http.
fn page_attempt(
    journal: &mut Journal,
    results: &mut Results,
    from: usize,
    (index, target): (usize, &str),
) {
    let alive = journal
        .since(from, |e| is_report(e, "http", "nav-alive", None))
        .into_iter()
        .find_map(|e| match e {
            Event::Report { body, .. }
                if body.get("i").and_then(serde_json::Value::as_u64) == Some(index as u64) =>
            {
                body.get("href").and_then(|h| h.as_str()).map(str::to_owned)
            }
            _ => None,
        });
    let filtered = refused(journal, from, target);
    let by = if filtered {
        "the filter"
    } else {
        "WebKit, before the filter was asked"
    };
    results.record(
        alive
            .as_deref()
            .is_some_and(|href| href.starts_with("http://127.0.0.1")),
        &format!("nav-page-{}", scheme(target)),
        format!("page set location to {target}; still on {alive:?}; refused by {by}"),
    );
}

/// Rust asks the web view to load `target`; the filter must refuse and the URL stay http.
async fn rust_attempt(view: &Webview, journal: &mut Journal, results: &mut Results, target: &str) {
    let from = journal.mark();
    let asked = Url::parse(target)
        .map_err(|e| e.to_string())
        .and_then(|url| view.navigate(url).map_err(|e| e.to_string()));
    journal.settle(Duration::from_millis(400)).await;
    let filtered = refused(journal, from, target);
    let still = view
        .url()
        .map(|u| u.scheme().to_owned())
        .unwrap_or_default();
    results.record(
        asked.is_ok() && filtered && device::is_web_scheme(&view.url().unwrap_or_else(|_| Url::parse("about:blank").expect("about:blank"))),
        &format!("nav-filter-{}", scheme(target)),
        format!("navigate({target}): asked {asked:?}, refused by the filter {filtered}, web view scheme now {still}"),
    );
}

fn refused(journal: &mut Journal, from: usize, target: &str) -> bool {
    let prefix: String = target.chars().take(20).collect();
    !journal
        .since(from, |e| matches!(e, Event::Navigation { label, url, allowed: false } if label == LABEL && url.starts_with(&prefix)))
        .is_empty()
}

fn scheme(target: &str) -> &str {
    target.split(':').next().unwrap_or(target)
}
