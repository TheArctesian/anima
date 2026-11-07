//! The four OD-7 checks, run in order against hidden web views. Each prints its lines as soon
//! as it is decided, so a timeout still shows what was proven.

use std::fmt::Display;
use std::time::Duration;

use tauri::window::WindowBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Url, WebviewBuilder, WebviewUrl, Window};

use crate::event::{Event, Events, Journal};
use crate::server::{self, Site};
use crate::trust::Pins;

mod device;
mod ipc;
mod navigation;
mod store;
mod tls;

/// Per-run random values, so stale data from an earlier run cannot pass a check.
pub struct Nonces {
    pub device: String,
    pub default: String,
}

/// What every check needs: the hidden window, the plain site, the pins and the event sink.
pub struct Harness {
    pub window: Window,
    pub events: Events,
    pub http: Site,
    pub pins: Pins,
}

impl Harness {
    pub fn http_url(&self, path_and_query: &str) -> Url {
        Url::parse(&format!(
            "http://127.0.0.1:{}{path_and_query}",
            self.http.port
        ))
        .expect("a loopback URL parses")
    }
}

#[derive(Default)]
pub struct Results {
    failed: usize,
    total: usize,
}

impl Results {
    pub fn record(&mut self, passed: bool, check: &str, detail: impl Display) {
        self.total += 1;
        if !passed {
            self.failed += 1;
        }
        println!("{} {check} {detail}", if passed { "PASS" } else { "FAIL" });
    }
}

pub async fn run(app: AppHandle, events: Events, mut journal: Journal, nonces: Nonces) -> i32 {
    let harness = match prepare(app, events, &mut journal).await {
        Ok(harness) => harness,
        Err(reason) => {
            println!("FAIL setup {reason}");
            return 1;
        }
    };
    let mut results = Results::default();
    main_view(&mut journal, &mut results);
    store::check(&harness, &mut journal, &mut results, &nonces).await;
    ipc::check(&harness, &mut journal, &mut results).await;
    tls::check(&harness, &mut journal, &mut results).await;
    navigation::check(&harness, &mut journal, &mut results).await;
    harness.http.stop().await;
    println!(
        "od7: {} of {} checks passed",
        results.total - results.failed,
        results.total
    );
    i32::from(results.failed > 0)
}

async fn prepare(app: AppHandle, events: Events, journal: &mut Journal) -> Result<Harness, String> {
    let http = server::serve_http(events.clone()).map_err(|e| e.to_string())?;
    let window = WindowBuilder::new(&app, "harness")
        .title("OD-7 spike")
        .inner_size(800.0, 600.0)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| format!("window: {e}"))?;
    let from = journal.mark();
    let main = WebviewBuilder::new("main", WebviewUrl::App("index.html".into())).focused(false);
    window
        .add_child(
            main,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(800.0, 600.0),
        )
        .map_err(|e| format!("main web view: {e}"))?;
    let started = journal
        .until(
            from,
            Duration::from_secs(10),
            |e| matches!(e, Event::Invoked { label, .. } if label == "main"),
        )
        .await;
    if started.is_none() {
        eprintln!("od7: the main page did not call the spike command within 10 s");
    }
    // After the first web view: WebKit crashes removing a store before it has started.
    device::clear_stores(&app).await;
    Ok(Harness {
        window,
        events,
        http,
        pins: Pins::default(),
    })
}

/// Positive control for check 2: the app's own page reaches the command.
fn main_view(journal: &mut Journal, results: &mut Results) {
    let calls: Vec<String> = journal
        .since(
            0,
            |e| matches!(e, Event::Invoked { label, .. } if label == "main"),
        )
        .into_iter()
        .filter_map(|e| match e {
            Event::Invoked { from, .. } => Some(from),
            _ => None,
        })
        .collect();
    results.record(
        !calls.is_empty(),
        "ipc-main-allowed",
        format!("the command ran for label main, from {calls:?}"),
    );
}
