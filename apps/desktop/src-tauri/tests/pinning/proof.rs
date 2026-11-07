//! The proof's steps. Each tab is opened through `open_tab` and pinned through `pin_tab`, as
//! the client's commands do; what the hook decided is read from the `web-view` events and
//! from what the site served.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    process,
    time::Duration,
};

use serde_json::{Value, json};
use tauri::{AppHandle, Listener, Manager, Url, Webview, window::WindowBuilder};
use tokio::{sync::mpsc, time::sleep};

use crate::{
    freed,
    site::Site,
    web_view::{
        Fingerprint,
        commands::{open_tab, pin_tab},
    },
};

/// The whole proof's limit; past it, it fails with status 2.
const LIMIT: Duration = Duration::from_secs(120);
/// How long one step waits for its event.
const WAIT: Duration = Duration::from_secs(10);
/// How long a stopped load is watched for a request that should not come.
pub const SETTLE: Duration = Duration::from_secs(1);
const EVENT: &str = "web-view";
/// One fixed device, so every run reuses one web store.
const DEVICE: &str = "5e0f7a52-31d4-4b69-9c1e-000000000431";

/// Runs the proof in a hidden Tauri app and exits with its result.
pub fn run() {
    std::thread::spawn(|| {
        std::thread::sleep(LIMIT);
        println!(
            "FAIL timeout the proof did not finish in {} s",
            LIMIT.as_secs()
        );
        process::exit(2);
    });
    let app = tauri::Builder::default()
        .manage(crate::web_view::Tabs::default())
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let passed = prove(&handle).await.unwrap_or_else(|reason| {
                    println!("FAIL setup {reason}");
                    false
                });
                process::exit(i32::from(!passed));
            });
            Ok(())
        })
        .build(tauri::generate_context!("tests/pinning/tauri.conf.json"))
        .expect("the proof's Tauri app builds");
    app.run(|_, _| {});
}

/// The `web-view` events, as they arrive.
struct Journal(mpsc::Receiver<Value>);

impl Journal {
    fn listen(app: &AppHandle) -> Self {
        let (sent, received) = mpsc::channel(256);
        app.listen_any(EVENT, move |event| {
            if let Ok(payload) = serde_json::from_str(event.payload()) {
                // A full journal drops the event; the step waiting for it then fails.
                let _full = sent.try_send(payload);
            }
        });
        Self(received)
    }

    fn clear(&mut self) {
        while self.0.try_recv().is_ok() {}
    }

    /// The next `kind` event of tab `tab`, within [`WAIT`].
    async fn next(&mut self, tab: &str, kind: &str) -> Option<Value> {
        let found = async {
            while let Some(event) = self.0.recv().await {
                if event["tabId"] == tab && event["kind"] == kind {
                    return Some(event);
                }
            }
            None
        };
        tokio::time::timeout(WAIT, found).await.ok().flatten()
    }
}

fn fingerprint(event: Option<&Value>, field: &str) -> Option<Fingerprint> {
    event?.get(field)?.as_str()?.parse().ok()
}

/// One device tab, opened as the client opens it.
pub struct Tab {
    pub app: AppHandle,
    pub id: &'static str,
    url: Url,
}

impl Tab {
    async fn open(app: &AppHandle, id: &'static str, url: &str) -> Result<Self, String> {
        let body = json!({
            "tabId": id,
            "deviceId": DEVICE,
            "url": url,
            "pin": null,
            "bounds": { "x": 0, "y": 0, "width": 800, "height": 600 },
        });
        open_tab(app, &body)
            .await
            .map_err(|error| error.code().to_owned())?;
        let url = Url::parse(url).map_err(|error| error.to_string())?;
        Ok(Self {
            app: app.clone(),
            id,
            url,
        })
    }

    pub fn view(&self) -> Result<Webview, String> {
        self.app
            .get_webview(&format!("web-{}", self.id))
            .ok_or_else(|| "the tab's web view is gone".to_owned())
    }

    fn reload(&self) -> Result<(), String> {
        self.view()?
            .navigate(self.url.clone())
            .map_err(|error| error.to_string())
    }

    fn pin(&self, leaf: Fingerprint) -> Result<(), String> {
        pin_tab(&self.app, self.id, &leaf.to_string()).map_err(|error| error.code().to_owned())
    }
}

/// Every check's line, and whether all passed.
#[derive(Default)]
pub struct Proof {
    failed: bool,
}

impl Proof {
    pub fn check(&mut self, name: &str, passed: bool, detail: &str) {
        println!("{} {name} {detail}", if passed { "PASS" } else { "FAIL" });
        self.failed |= !passed;
    }
}

async fn prove(app: &AppHandle) -> Result<bool, String> {
    WindowBuilder::new(app, "main")
        .visible(false)
        .build()
        .map_err(|error| error.to_string())?;
    let mut journal = Journal::listen(app);
    let mut proof = Proof::default();
    loopback_v4(app, &mut journal, &mut proof).await?;
    loopback_v6(app, &mut journal, &mut proof).await?;
    Ok(!proof.failed)
}

/// No pin, then the pin, then a new certificate on the same host and port, then the same after
/// the view moves to a window of its own.
async fn loopback_v4(
    app: &AppHandle,
    journal: &mut Journal,
    proof: &mut Proof,
) -> Result<(), String> {
    let address = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let first = Site::serve(address, 0, false)?;
    let tab = Tab::open(app, "v4", &format!("https://127.0.0.1:{}/", first.port)).await?;
    first_visit(&tab, journal, proof, &first).await;
    pinned_visit(&tab, journal, proof, &first).await?;
    let (port, pinned) = (first.port, first.leaf);
    first.stop().await;
    let second = restart(address, port).await?;
    mismatch_visit(&tab, journal, proof, (pinned, &second)).await?;
    let window = WindowBuilder::new(app, "pop-v4")
        .visible(false)
        .build()
        .map_err(|error| error.to_string())?;
    // `pop_out`'s move, in a window that stays hidden: the same WKWebView, a new parent.
    tab.view()?
        .reparent(&window)
        .map_err(|error| error.to_string())?;
    journal.clear();
    tab.reload()?;
    let mismatch = journal.next(tab.id, "MISMATCH").await;
    sleep(SETTLE).await;
    let stopped = fingerprint(mismatch.as_ref(), "presented") == Some(second.leaf);
    proof.check(
        "mismatch-cancelled-after-pop-out",
        stopped && second.served() == 0,
        &format!(
            "event {mismatch:?}; pages served on the new certificate {}",
            second.served()
        ),
    );
    second.stop().await;
    Ok(())
}

/// An IPv6 tab spelled long-hand, on a site whose leaf is signed by an issuer: the hook must
/// meet `WebKit`'s spelling of the host and show the leaf, not the issuer. Then the pinned
/// view must keep its scheme filter, and must be freed when its tab closes.
async fn loopback_v6(
    app: &AppHandle,
    journal: &mut Journal,
    proof: &mut Proof,
) -> Result<(), String> {
    let site = Site::serve(IpAddr::V6(Ipv6Addr::LOCALHOST), 0, true)?;
    let url = format!("https://[0:0:0:0:0:0:0:1]:{}/", site.port);
    let tab = Tab::open(app, "v6", &url).await?;
    let untrusted = journal.next(tab.id, "UNTRUSTED").await;
    let shown = fingerprint(untrusted.as_ref(), "fingerprintSha256");
    journal.clear();
    tab.pin(site.leaf)?;
    let loaded = journal.next(tab.id, "LOADED").await;
    proof.check(
        "ipv6-chain-pinned-load",
        shown == Some(site.leaf) && loaded.is_some() && site.served() > 0,
        &format!(
            "leaf {}; shown {shown:?}; loaded {loaded:?}; pages served {}",
            site.leaf,
            site.served()
        ),
    );
    forbidden_schemes(&tab, proof).await?;
    freed::closed_tab_freed(&tab, proof).await?;
    site.stop().await;
    Ok(())
}

/// The proxy still hands navigation to wry's filter: a page cannot leave for another scheme.
async fn forbidden_schemes(tab: &Tab, proof: &mut Proof) -> Result<(), String> {
    let view = tab.view()?;
    for target in ["data:text/html,proof", "ipc://localhost/web_view_open"] {
        let url = Url::parse(target).map_err(|error| error.to_string())?;
        view.navigate(url).map_err(|error| error.to_string())?;
        sleep(SETTLE).await;
        let now = view.url().map_err(|error| error.to_string())?;
        proof.check(
            "forbidden-scheme-refused",
            now.scheme() == "https",
            &format!("asked for {target}; the view is on scheme {}", now.scheme()),
        );
    }
    Ok(())
}

/// No pin: the leaf reaches the client and nothing loads.
async fn first_visit(tab: &Tab, journal: &mut Journal, proof: &mut Proof, site: &Site) {
    let untrusted = journal.next(tab.id, "UNTRUSTED").await;
    sleep(SETTLE).await;
    let shown = fingerprint(untrusted.as_ref(), "fingerprintSha256");
    proof.check(
        "untrusted-shown",
        shown == Some(site.leaf) && site.served() == 0,
        &format!(
            "site leaf {}; event {untrusted:?}; pages served {}",
            site.leaf,
            site.served()
        ),
    );
}

/// With the pin: the same certificate loads.
async fn pinned_visit(
    tab: &Tab,
    journal: &mut Journal,
    proof: &mut Proof,
    site: &Site,
) -> Result<(), String> {
    journal.clear();
    tab.pin(site.leaf)?;
    let loaded = journal.next(tab.id, "LOADED").await;
    proof.check(
        "pinned-load",
        loaded.is_some() && site.served() > 0,
        &format!("event {loaded:?}; pages served {}", site.served()),
    );
    Ok(())
}

/// R-069: a new certificate on the pinned host and port is cancelled and shown.
async fn mismatch_visit(
    tab: &Tab,
    journal: &mut Journal,
    proof: &mut Proof,
    (pinned, site): (Fingerprint, &Site),
) -> Result<(), String> {
    journal.clear();
    tab.reload()?;
    let mismatch = journal.next(tab.id, "MISMATCH").await;
    sleep(SETTLE).await;
    let expected = fingerprint(mismatch.as_ref(), "expected");
    let presented = fingerprint(mismatch.as_ref(), "presented");
    proof.check(
        "mismatch-cancelled",
        expected == Some(pinned) && presented == Some(site.leaf) && site.served() == 0,
        &format!(
            "pinned {pinned}; new {}; event {mismatch:?}; pages served on the new certificate {}",
            site.leaf,
            site.served()
        ),
    );
    Ok(())
}

/// The port may take a moment to free after the old site's sockets close.
async fn restart(address: IpAddr, port: u16) -> Result<Site, String> {
    let mut last = String::new();
    for _ in 0..20 {
        match Site::serve(address, port, false) {
            Ok(site) => return Ok(site),
            Err(error) => last = error,
        }
        sleep(Duration::from_millis(100)).await;
    }
    Err(format!("no restart on {port}: {last}"))
}
