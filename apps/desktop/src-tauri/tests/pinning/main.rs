//! R-069, WEB-008, OD-7: the macOS trust hook end to end, in a real `WKWebView` that the shell's
//! own `open_tab` and `pin_tab` drive, against HTTPS sites on loopback with fresh self-signed
//! certificates. It proves that an untrusted certificate is stopped and shown, that the pinned
//! one loads, and that a mismatched one is cancelled, also after the view moves window.
//!
//! Not libtest (`harness = false`): Tauri's event loop must own the main thread, and libtest
//! runs every test on a thread of its own. `cargo test -p anima-desktop` runs it with the rest;
//! alone: `cargo test -p anima-desktop --test pinning`. It prints `PASS|FAIL <check> <detail>`
//! and exits 0 only if every check passed. Every window stays hidden and the app takes the
//! Accessory policy, so it neither shows nor takes the screen.
//!
//! The web view module is compiled in from the shell's source, so the proof runs the
//! production code, not a copy.

#[cfg(target_os = "macos")]
mod proof;
#[cfg(target_os = "macos")]
mod site;
/// The shell's source folder: `web_view` and its folder load from there as in the shell.
/// Cargo builds this target with `--cfg test` but no libtest, so the module's unit tests
/// compile without their `#[test]` functions; with the commands the proof does not call,
/// that code is unused here, and only here.
#[cfg(target_os = "macos")]
#[path = "../../src"]
#[expect(
    dead_code,
    unused_imports,
    clippy::unused_async,
    reason = "the proof drives the web view module through open_tab and pin_tab only"
)]
mod shell {
    pub mod web_view;
}
#[cfg(target_os = "macos")]
use shell::web_view;

#[cfg(target_os = "macos")]
fn main() {
    proof::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("pinning: skipped, only macOS has the trust hook");
}
