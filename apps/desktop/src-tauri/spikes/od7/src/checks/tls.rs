//! Check 3 (WEB-008): the untrusted leaf's SHA-256 reaches Rust and equals the server's; with
//! that pin the page loads; a new certificate on the same host and port is stopped.

use std::time::Duration;

use tauri::{Url, Webview};

use super::device::{self, STORE_TLS};
use super::{Harness, Results};
use crate::event::{Event, Journal, is_report};
use crate::pin::{Fingerprint, Pin, Verdict};
use crate::server::{self, Site};
use crate::trust::{self, TrustHook};

const LABEL: &str = "web-tls";
const WAIT: Duration = Duration::from_secs(8);

pub async fn check(harness: &Harness, journal: &mut Journal, results: &mut Results) {
    if let Err(reason) = run(harness, journal, results).await {
        results.record(false, "tls-setup", reason);
    }
}

/// The TLS web view and the HTTPS port every step visits.
struct Visit {
    view: Webview,
    port: u16,
}

impl Visit {
    fn go(&self, journal: &mut Journal, step: u8) -> Result<usize, String> {
        let from = journal.mark();
        let url = format!("https://127.0.0.1:{}/tls?step={step}", self.port);
        let url = Url::parse(&url).map_err(|e| e.to_string())?;
        self.view.navigate(url).map_err(|e| e.to_string())?;
        Ok(from)
    }
}

async fn run(
    harness: &Harness,
    journal: &mut Journal,
    results: &mut Results,
) -> Result<(), String> {
    let first = server::serve_tls(0, 1, harness.events.clone()).map_err(|e| e.to_string())?;
    let first_fp = first.fingerprint.ok_or("no fingerprint")?;
    let visit = Visit {
        view: hooked_view(harness, journal).await?,
        port: first.port,
    };
    first_visit(&visit, journal, results, first_fp).await?;
    let pin = Pin {
        host: "127.0.0.1".into(),
        port: visit.port,
        sha256: first_fp,
    };
    harness
        .pins
        .lock()
        .map_err(|_| "pins poisoned")?
        .insert(LABEL.into(), pin);
    pinned_visit(&visit, journal, results).await?;
    first.stop().await;
    let second = restart(visit.port, harness).await?;
    let second_fp = second.fingerprint.ok_or("no fingerprint")?;
    switched_visit(&visit, journal, results, (first_fp, second_fp)).await?;
    second.stop().await;
    Ok(())
}

/// No pin: the leaf must reach Rust, equal the server's, and the page must not load.
async fn first_visit(
    visit: &Visit,
    journal: &mut Journal,
    results: &mut Results,
    leaf: Fingerprint,
) -> Result<(), String> {
    let from = visit.go(journal, 1)?;
    let challenge = journal.until(from, WAIT, is_challenge).await;
    journal.settle(Duration::from_millis(500)).await;
    let served = served_on(journal, from, "tls-1");
    let seen = verdict_from(challenge.as_ref(), visit.port);
    results.record(
        matches!(seen, Some(Verdict::Untrusted(fp)) if fp == leaf) && served == 0,
        "tls-fingerprint",
        format!("server leaf {leaf}; challenge {challenge:?}; pages served {served}"),
    );
    Ok(())
}

/// With the pin: the same certificate now loads and the page's script runs.
async fn pinned_visit(
    visit: &Visit,
    journal: &mut Journal,
    results: &mut Results,
) -> Result<(), String> {
    let from = visit.go(journal, 2)?;
    let loaded = journal
        .until(from, WAIT, |e| is_report(e, "tls-1", "tls-loaded", None))
        .await;
    let accepted = journal
        .since(from, |e| {
            matches!(
                e,
                Event::Challenge {
                    verdict: Verdict::Accept,
                    ..
                }
            )
        })
        .len();
    results.record(
        loaded.is_some() && accepted > 0,
        "tls-pinned-load",
        format!(
            "page ran under the pinned leaf: {}; accepted challenges {accepted}",
            loaded.is_some()
        ),
    );
    Ok(())
}

/// Same host and port, new certificate: WebKit must ask again and the pin must stop it.
async fn switched_visit(
    visit: &Visit,
    journal: &mut Journal,
    results: &mut Results,
    (pinned, new): (Fingerprint, Fingerprint),
) -> Result<(), String> {
    let from = visit.go(journal, 3)?;
    let challenge = journal.until(from, WAIT, is_challenge).await;
    journal.settle(Duration::from_secs(1)).await;
    let served = served_on(journal, from, "tls-2");
    let port = visit.port;
    results.record(
        challenge.is_some(),
        "tls-rechallenge",
        format!("a challenge after the certificate changed on 127.0.0.1:{port}: {challenge:?}"),
    );
    let stopped = matches!(verdict_from(challenge.as_ref(), port),
        Some(Verdict::Mismatch { expected, presented }) if expected == pinned && presented == new);
    results.record(
        stopped && served == 0 && pinned != new,
        "tls-mismatch-stopped",
        format!("expected {pinned} presented {new}; pages served on the new certificate {served}"),
    );
    Ok(())
}

/// A device web view on a blank http page, with the trust hook in place before any https load.
async fn hooked_view(harness: &Harness, journal: &mut Journal) -> Result<Webview, String> {
    let from = journal.mark();
    let view = device::open(harness, LABEL, Some(STORE_TLS), harness.http_url("/blank"))?;
    let hook = TrustHook::new(LABEL, harness.pins.clone(), harness.events.clone());
    let events = harness.events.clone();
    view.with_webview(move |platform| {
        let result = trust::install(&platform, hook);
        events.send(Event::Hook { result });
    })
    .map_err(|e| e.to_string())?;
    let installed = journal
        .until(from, WAIT, |e| matches!(e, Event::Hook { .. }))
        .await;
    match installed {
        Some(Event::Hook {
            result: Ok(detail), ..
        }) => {
            println!("od7: trust hook: {detail}");
            Ok(view)
        }
        Some(Event::Hook {
            result: Err(reason),
            ..
        }) => Err(format!("trust hook: {reason}")),
        _ => Err("trust hook: no answer".into()),
    }
}

/// The port may take a moment to free after the old site's sockets close.
async fn restart(port: u16, harness: &Harness) -> Result<Site, String> {
    let mut last = String::new();
    for _ in 0..20 {
        match server::serve_tls(port, 2, harness.events.clone()) {
            Ok(site) => return Ok(site),
            Err(e) => last = e.to_string(),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!("restart on {port}: {last}"))
}

fn is_challenge(event: &Event) -> bool {
    matches!(event, Event::Challenge { label, .. } if label == LABEL)
}

/// The verdict of a challenge for 127.0.0.1:`port` that the system did not trust.
fn verdict_from(event: Option<&Event>, port: u16) -> Option<Verdict> {
    match event {
        Some(Event::Challenge {
            host,
            port: p,
            verdict,
            system_trusted: false,
            ..
        }) if host == "127.0.0.1" && *p == port => Some(verdict.clone()),
        _ => None,
    }
}

fn served_on(journal: &mut Journal, from: usize, site: &str) -> usize {
    journal
        .since(
            from,
            |e| matches!(e, Event::Served { server, path, .. } if server == site && path == "/tls"),
        )
        .len()
}
