# WP-31 report — Desktop web view and web tab

Status: done. Branch `claude/wp-31-web-view`. Tasks: T1 09b4b81 (merged to phase-2 at de7cffc), T2–T4 stack
60113cc, T5 48eb0d2. Plan and review log: [WP-31.md](WP-31.md). Rulings used: R-066, R-067, R-068, R-069.

## OD-7

- **macOS (WKWebView): pinning works**, proven by the spike (20/20) and by the shell's own harness=false
  proof `cargo test -p anima-desktop --test pinning` (untrusted shown, pinned load, mismatch cancelled,
  mismatch after pop-out, IPv6 with a CA-signed leaf, forbidden scheme refused, closed view freed).
  It needs `unsafe` Objective-C: one proxy `WKNavigationDelegate` in `src/web_view/macos_trust.rs` (R-069).
- **Linux (WebKitGTK): not tested.** The spike's Linux path is written, not compiled. Linux answers
  `web_view_pinning() = false`, so https interfaces use the system browser with the pinned fingerprint
  (D-9); http embeds.

## What changed

- `apps/desktop/src-tauri/`: `web_view/` (child web views per tab, data store per device, http/https-only
  navigation, no new windows, pop-out, `web-view` event to `main` only), `web_view/trust/` (fingerprint,
  pinned origin, D-4 verdict, leaf probe), `macos_trust.rs`, `system_browser.rs`, ten commands allowed for
  `main` only; crate lints copied from the workspace with `unsafe_code = "deny"`; tauri `unstable`.
- `apps/client/src/lib/desktop/webView/`: shell wrappers and a strict event parser.
- `apps/client/src/lib/sessions/web/`: `WebTab` (toolbar, view area, fingerprint prompt and accept through
  `acceptWebCertificate`, mismatch stop, Down and failed pages with Reload, 30 s load timeout, D-9 page).
- `apps/desktop/src-tauri/spikes/od7/`: the standalone OD-7 harness (outside the workspace).

## Edits outside owned paths (PKG-001)

- `apps/client/src/lib/hub/generation.ts`: codegen also reads `src/lib/*/*/api.ts` (one line), and the
  regenerated `lib/hub/generated/`. Alternative: move the `acceptWebCertificate` document into WP-26's
  `lib/web-interfaces/api.ts` and revert the glob.
- `Cargo.lock`: new edges only (no new package versions).

## Handoffs

- **WP-24 (strip):** a web tab kind in `tabs.ts` holding `{ id, deviceId, url, label,
  pinnedFingerprintSha256 }`, tab ids `[A-Za-z0-9_-]{1,128}`; a `SessionPanes` branch rendering `WebTab`
  with a live `device.status`; `overlayOpen = true` whenever a menu, popover or modal covers the pane (the
  native view sits above the DOM, MAT-008); a definite pane height; no motion or blur over the pane.
- **WP-26 (Web button):** `ANIMA_TAB` opens the web tab; `SYSTEM_BROWSER` http calls
  `openSystemBrowser`; `SYSTEM_BROWSER` https goes through the web tab, which reads and compares the pin.
- **Programme:** `__TAURI_INTERNALS__` is still injected into device web views and `ipc://` reaches Tauri;
  the capability ACL is the only barrier (probed: 38 commands, device labels 0 allowed, also after
  pop-out). Keep every capability `"webviews": ["main"]`, no `windows`, no `remote`. Could: a `caller()`
  check on `access_token_*`.

## Needs the operator

- Run the spike on Linux (`apps/desktop/src-tauri/spikes/od7`, README: `xvfb-run -a cargo run`); if it
  passes, a Linux trust hook (WebKitGTK, safe Rust) is a follow-up and Linux can answer `true`.
- Criterion 9 live: a real device web interface on an untrusted certificate in the desktop app (first visit
  prompt, accept, pin, a changed certificate stops). Pin over a system-trusted certificate (D-4) was not
  observed live (needs a trusted root).

## Open coulds (issue #1)

- Closed tab challenge answers Cancel (m15) has no test; proxy could answer Cancel itself if wry's delegate
  is gone (unproven crash path).
- No load-failure hook: unreachable devices show the failed page after the 30 s timeout; no event when a
  popped-out window is closed (found at the next command).
- Navigation filter refuses `about:`/`data:`/`blob:` frames (strict D-3); some device UIs may need a ruling.
- `SecTrustEvaluateWithError` runs on the main thread for unpinned tabs.
- Commands are not run in tests (they take a Wry `Webview`); MOT-006 background-colour transitions as in
  neighbours; Chromium CLS 0.0129 on the Down page at 960 (content mounts after `pinning()`).
