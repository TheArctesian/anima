//! OD-7 spike harness (WP-31 T1). Runs hidden, prints `PASS|FAIL <check> <detail>` per check
//! and exits 0 only if every check passed. A hard 60-second limit ends it with status 2.

mod checks;
mod event;
mod pages;
mod pin;
mod server;
mod trust;

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::Duration;

use event::{Event, Events};

const LIMIT: Duration = Duration::from_secs(60);

/// The spike command. Records which web view reached it; nothing else.
#[tauri::command]
fn spike_probe(
    webview: tauri::Webview,
    events: tauri::State<'_, Events>,
    from: String,
) -> &'static str {
    events.send(Event::Invoked {
        label: webview.label().to_owned(),
        from,
    });
    "ok"
}

fn main() {
    std::thread::spawn(|| {
        std::thread::sleep(LIMIT);
        println!(
            "FAIL timeout the harness did not finish in {} s",
            LIMIT.as_secs()
        );
        std::process::exit(2);
    });
    let (events, journal) = event::channel();
    let nonces = checks::Nonces {
        device: nonce(),
        default: nonce(),
    };
    tauri::Builder::default()
        .manage(events.clone())
        .invoke_handler(tauri::generate_handler![spike_probe])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let code = checks::run(handle, events, journal, nonces).await;
                std::process::exit(code);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the Tauri app runs");
}

/// 16 random hex digits from the standard library's per-process random hash keys.
fn nonce() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u8(0);
    format!("{:016x}", hasher.finish())
}
