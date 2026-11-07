# OD-7 spike: device web views in Tauri 2

WP-31 task T1 (decision D-1). A harness that answers OD-7 for the Tauri child web view
(`Window::add_child`, tauri 2.12.5, wry 0.57.1): can a device web view

1. use its own data store (WEB-006),
2. be denied every Tauri command (WEB-007),
3. report an untrusted certificate's leaf SHA-256 to Rust, load with that pin, and stop when
   a different certificate appears on the same host and port (WEB-008),
4. refuse navigation to anything but `http` and `https` (D-3)?

It is a standalone crate: the empty `[workspace]` keeps it out of the Anima workspace, whose
`unsafe_code = "forbid"` it cannot meet (the macOS trust hook is Objective-C). No gate runs on
it. `Cargo.lock` is a copy of the workspace lock, so versions match the shell.

## What it does

The harness runs hidden: one window with `visible(false)`, macOS activation policy `Accessory`
(no Dock icon, no focus), all web views `focused(false)`. Two local servers listen on
`127.0.0.1` only: a plain HTTP site and an HTTPS site with a fresh self-signed `rcgen`
certificate. Pages report to the server they came from by `fetch`, never by IPC. Rust records
every server hit, page report, command call, navigation decision and certificate challenge in
order, then decides each check. Every line is `PASS|FAIL <check> <detail>`; the exit status is
0 only if all pass, 1 if any fails, 2 if the 60-second limit is reached.

| Check | What it proves |
| --- | --- |
| `ipc-main-allowed` | Control: the app's own page (`tauri://localhost`, label `main`) can call the spike command. |
| `store-own-data` | Control: device A's store and the default store each read back the cookie (script and `Set-Cookie`) and `localStorage` they set. |
| `store-device-isolated` | Device B (its own store) sees neither A's nor the default store's data, in `document.cookie`, `localStorage` or the `Cookie` header, on the same origin. |
| `store-device-vs-main` | Device A does not see the default store (the one the main view uses) and the default store does not see A. |
| `ipc-device-denied` | A device page that tries `__TAURI_INTERNALS__.invoke`, a raw `webkit.messageHandlers.ipc` message and a `fetch` to `ipc://localhost/` never runs the command (Rust records the caller's label). |
| `tls-fingerprint` | With no pin, the trust hook receives the challenge for `127.0.0.1:<port>`, the system does not trust it, the leaf SHA-256 equals the server's own, and no page is served. |
| `tls-pinned-load` | With that fingerprint pinned, the hook accepts and the page's script runs. |
| `tls-rechallenge` | After the server restarts on the same port with a new certificate, WebKit asks the hook again. |
| `tls-mismatch-stopped` | That challenge is a mismatch (expected the pin, presented the new leaf) and no page is served on the new certificate. |
| `nav-page-<scheme>` | A device page that sets `location` to `file:`, `data:`, `tauri:`, `ipc:` or an unknown scheme is still on its `http` page afterwards. The detail says whether the filter or WebKit refused it. |
| `nav-http-allowed` | Control: the same page can then go to an `http` URL. |
| `nav-filter-<scheme>` | Rust's `navigate()` to each target is refused by the filter and the web view stays on `http`. |

Store identifiers are fixed 16-byte values; the harness removes their stores at start (after
the first web view exists: WebKit crashes if a store is removed before it has started), and
each run uses fresh random values, so data from an earlier run cannot pass a check.

## Run on macOS (14 or later)

```sh
cd apps/desktop/src-tauri/spikes/od7
cargo run
```

Builders in a WP-31 worktree use the shared target and the package's cargo lock (R-068):

```sh
CARGO_TARGET_DIR=<repo>/.claude/worktrees/wp-31/target <cargo-locked.sh> cargo run --offline
```

The trust hook (`src/trust/macos.rs`) is a navigation-delegate proxy. wry's
`WryNavigationDelegate` does not implement
`webView:didReceiveAuthenticationChallenge:completionHandler:`, and neither wry nor Tauri
offers a certificate API. The proxy implements that one method, forwards every other selector
to wry's delegate (`respondsToSelector:` and `forwardingTargetForSelector:`), is installed with
`setNavigationDelegate:` from `Webview::with_webview`, and is kept alive by the harness because
`navigationDelegate` is a weak property.

## Run on Linux (WebKitGTK)

Written but not yet compiled or run: the builder had only macOS. `src/trust/linux.rs` connects
`load-failed-with-tls-errors`, takes the leaf's DER from the `GTlsCertificate`, and for a
pinned leaf calls `WebContext::allow_tls_certificate_for_host` and loads the URI again. Each
device store is a `data_directory` under the temp folder (`anima-od7-spike/<id>`).

```sh
# Debian / Ubuntu build dependencies for Tauri 2 (as in the Tauri prerequisites)
sudo apt install build-essential libwebkit2gtk-4.1-dev libxdo-dev librsvg2-dev
cd apps/desktop/src-tauri/spikes/od7
cargo run                 # in a desktop session
xvfb-run -a cargo run     # headless
```

If it does not compile, the most likely places are `src/trust/linux.rs` (webkit2gtk-rs 2.0
signal and property signatures) and `with_store` in `src/checks/device.rs`. Known limits of the
Linux path, whatever the result: the allowance is per host (not per port) and per web context,
and the signal never fires for a certificate the system trusts, so "a pin wins over system
trust" (D-4) is not enforced there.

## Files

- `src/main.rs` wiring, the spike command, the 60-second limit
- `src/checks.rs`, `src/checks/*.rs` the checks; `device.rs` opens device web views (store, filter)
- `src/event.rs` the ordered record of what happened
- `src/server.rs`, `src/pages.rs` the loopback test sites and their pages
- `src/pin.rs` the pure pin decision (D-4) and fingerprint form (D-5)
- `src/trust.rs`, `src/trust/{macos,linux}.rs` the platform trust hooks
