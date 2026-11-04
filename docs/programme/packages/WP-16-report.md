# WP-16 report (package orchestrator)

Status: **done**. Branch `claude/wp-16-tauri-shell`, with the latest `phase-1` (`ae17d36`) merged in. Not pushed, not merged.
`bun run gates` on the tip: **GATES_RESULT**. Plan, decisions D-1..D-8, handoffs and the review log: [WP-16.md](WP-16.md).

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Tauri 2 shell | `apps/desktop/src-tauri/`:<br>- Tauri 2.12.5 with default features off, plus `wry` and `compression`<br>- one `main` window<br>- strict CSP<br>- `withGlobalTauri` off<br>- no shell, fs or http plugins<br>- a workspace member, so gates 1–3 cover it; it compiles and tests with no `apps/client/build` (R-043)<br>- `apps/desktop/package.json` holds `@tauri-apps/cli` 2.12.4 | CLI-001, REPO-003, D-1, D-4, D-5, D-7 |
| Key store | `src/keystore/`:<br>- `access_token_read/write/clear({ origin, … })` through `keyring` 3.6.3 (macOS Keychain, Linux Secret Service)<br>- one entry per canonical hub origin, under service `ing.anima.desktop` (`.dev` in debug builds)<br>- typed errors `{ code: unavailable \| denied \| failed \| empty \| origin, message }` with fixed messages; the backend text and the token are dropped<br>- macOS cancel or auth failure maps to `denied`<br>- blocking calls run off the async runtime<br>- tests use an in-memory store, never the real Keychain | CLI-003, CLI-004, LOG-007, D-2 |
| Capabilities | `capabilities/main.json`: the `main` webview only, `core:default` plus the three generated `allow-access-token-*` permissions, no remote URLs | D-3 |
| Client key-store gateway | `apps/client/src/lib/desktop/`:<br>- `readAccessToken`, `writeAccessToken` and `clearAccessToken` take a `HubAddress` and refuse one whose `origin`, `http` and `socket` are not exactly what `parseHubAddress` makes<br>- outside Tauri, a read is `null` and a write or clear fails with `not-desktop`<br>- `storedHubAddress` / `storeHubAddress` keep the canonical origin in local storage | CLI-002, CLI-003, H-2, H-5 |
| Hub binding | `lib/hub`:<br>- `TokenProvider` is `(address) => …`, so the HTTP and socket transports read the token for the hub's own address on every request and reconnect<br>- `createHub` keeps a frozen copy of the address<br>- `parseHubAddress` and `DEFAULT_HUB_ADDRESS` are frozen | CLI-002, CLI-003 |
| Layout | `routes/+layout.svelte`: `createHub({ address: storedHubAddress(localStorage), token: readAccessToken })` | H-2 |
| Lint boundary | `eslint.config.js` append (P-4, R-043): `@tauri-apps/*` only in `$lib/desktop`, with rule tests | REPO-003 |
| Packages | macOS:<br>- `Anima.app` (17.1 MiB, `LSMinimumSystemVersion` 14.2)<br>- `Anima_0.1.0_aarch64.dmg` (10.3 MiB)<br>Linux:<br>- `Anima_0.1.0_aarch64.AppImage` (83.7 MiB), built in Docker on ubuntu 24.04 (WebKitGTK 2.52.6)<br>- it ran 20 s under Xvfb without exiting | Section 3, H-1 |

**Tests:**
- 14 shell tests (Rust)
- 389 client tests in total; the new ones cover the key-store gateway, address storage, token binding, layout wiring and lint rules

**Gate cost** (R-043):

| Gate | Cold | Warm |
| --- | --- | --- |
| clippy | 33 s | 6 s |
| cargo-test | 231 s | 52 s |

The Tauri release build takes 4 min 36 s cold on macOS. The Linux AppImage takes 13 min in a fresh container, including toolchain install.

## Review findings (summary; detail in WP-16.md)

**T1 shell and T2 key store.** Security review took 2 rounds and the second round caught all 29 mutations. Shoulds fixed:
- a macOS Deny mapped to `unavailable`;
- the commands were untested;
- dev mode loaded whatever held port 5173 (now `--strictPort` and a separate dev keyring service).

Coulds fixed: webview-scoped capabilities, a fuller CSP, a strict origin guard, and `_` allowed in host names.

**T3 client.** Security review took 4 rounds.
- Round 1, must: the token was not bound to its hub. A changed address sent the old hub's token to the new host, which a test proved. The fix is one entry per origin.
- Later rounds, shoulds: string origins were replaced by `HubAddress`. `keyOf` checked only `origin` while requests went to `http` and `socket`; a test proved the exploit with hostile endpoints.

**T3b token binding.** Added after T3 round 4. Security review took 2 rounds. It fixed two proven paths:
- `createHub` paired an address with a separately chosen token;
- an address object could be changed after creation.

The second round caught all 9 mutations.

**T4 package.** The orchestrator built both platforms. Watched-app checks went to the operator (below).

**Final package review.** Accept with fixes, no must. No other `createHub` or `TokenProvider` caller on phase-1 breaks. It found stale plan text and empty handoffs; both are fixed.

## Handoffs (PKG-001)

- **H-1 WP-15 (login).**
  - Write the token with `writeAccessToken(hub.address, token)`, using the hub that ran the login. Never use `DEFAULT_HUB_ADDRESS` or the stored text.
  - Call `clearAccessToken(hub.address)` on log out.
  - Use `storeHubAddress(localStorage, text)` for the address field. It takes effect after a reload or a recreated hub.
  - A custom `TokenProvider` must use its `address` argument.
  - Handle the problems `denied`, `unavailable` and `not-desktop`.
  - The three write-side exports have no caller yet: WP-15 uses them or they are deleted.
- **H-2 WP-04 (updater, release).**
  - Add `capabilities/desktop.json` and the updater config.
  - Build through `tauri build`. A plain `cargo build --release` loads `devUrl`.
  - Set `CI=true` for a headless DMG.
  - Use ubuntu 24.04+ with the packages listed in `quality/README.md`.
  - AppImage bundling downloads linuxdeploy and AppRun from GitHub.
  - x86_64 was not built here.
- **H-3 WP-14 (eslint).**
  - Append the `guacamole-common-js` rule separately from WP-16's.
  - Keep `tauriImports` in the `hub` and `motion` restrictions, and keep the `$lib/desktop` override.
- **H-4 operator (watched run of the packaged app).**
  - H-6: does Tab reach both nav links in WKWebView? Computer-use access was refused on this host, and release builds have no devtools.
  - Does the app run without `core:default` (C1)?
  - Does it run without `style-src 'unsafe-inline'`?
  - Do Keychain prompts repeat after a denial (C5)?

## Contract change requests (CON-001)

None to `contracts/`. The client-internal `TokenProvider` signature in `lib/hub` (WP-11) changed in T3b. That change is recorded here for the programme.

## Deferred (for issue #1, R-007)

- The four watched-app checks in H-4.
- The browser (web client) has no token provider (C2). `address.ts` may belong in `$lib/hub/` once the web client needs it (C4).
- TypeScript cannot refuse a `TokenProvider` that ignores its `address` argument. A lint rule could require `token:` to be an identifier.
- Export `AddressResult` from `$lib/hub/hub` for `storeHubAddress`'s return type (C-c). `AccessTokenProblem` could be unexported.
- The default panic hook prints a key-store thread panic to stderr; no secret goes there today (C-b).
- Linux Secret Service behaviour (locked collection, no daemon) was not exercised on a real desktop session. The macOS legacy Keychain ACL trusts only the creating app.
- CLI-001 and CLI-004 rest on review evidence. A config and credential-type check could make them tests.
- An x86_64 Linux build, and a launch check of the AppImage with a real compositor.
- Pop-out windows (WP-14 RA-002) are not built in WP-16. Build them to the port WP-14 passes: device identity `{ deviceId, name, protocol }`, not a live session. The new window starts its own session through `openSession`, and the tab's session ends normally.

## Notes

- The squashed T3 merge (`8cc69ad`) folded duplicate-message commits `44c1eb0` and `0c91193`. `bun.lock` was regenerated.
- In the phase-1 merge (`1d4b7dd`), `Cargo.lock` takes phase-1's lockfile with the desktop crate's dependencies resolved on top, at the reviewed versions.
- T1 commit `7caeaef` fails `cargo fmt`. The next commit, `d11eed3`, fixes it.
