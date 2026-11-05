# WP-04 stage B report (desktop releases, self-update, process log)

Status: **accepted** (final package review, reviewer-security). Branch `claude/wp-04b-releases` from `phase-1` `9a4623e`; not pushed, not merged. Plan, decisions, review log and handoffs: [WP-04b.md](WP-04b.md). Operator guide: [docs/releases.md](../../releases.md).

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Release workflow (the only workflow, R-005) | `.github/workflows/release.yml`: push to `main` and manual start only, and the `version` job refuses any ref but `main`. `cancel-in-progress: false`. The `release` environment is on all five jobs. Actions are pinned by SHA, and setup-bun runs with `no-cache`. Jobs: `version` → `build-macos` (universal, ad-hoc signed unless all six Apple secrets exist; `codesign --verify --deep --strict`, `lipo -archs` = x86_64 + arm64) ∥ `build-linux` (an unsigned AppImage) → `sign-linux` (a fresh runner, only the locked Tauri CLI, `signer sign --app-version`) → `publish` (`!cancelled()`; a draft release, every file, `latest.json` last, then published as latest). The key reaches only the macOS bundle step and the Linux sign step; `GH_TOKEN` reaches only the `gh release` steps. | UPD-007–012 |
| Feed | `deploy/release/feed.ts` (pure), `make-feed.ts` (CLI) and `signed-version.ts`. `latest.json` lists only the platforms that built and were signed in this run, with versioned URLs. It also writes `Anima.dmg`, `Anima-x86_64.AppImage` and `SHA256SUMS`. Every `.sig` must carry `version:<release version>`, or the run fails before the release is created. | UPD-008–010, UPD-002 |
| Shell | `apps/desktop/src-tauri`:<br>- `tauri-plugin-updater` 2.14.1 (rustls) and `tauri-plugin-process` 2.4.0, registered only in release desktop builds;<br>- `tauri-plugin-log` 2.10.1 writing to the OS log folder, rotated at 1 MiB × 5, with control and bidi characters escaped;<br>- the `process_log_folder_open` command (no arguments, `/usr/bin/open` / `xdg-open`, fixed errors);<br>- `capabilities/desktop.json`;<br>- `tauri.conf.json`: `createUpdaterArtifacts`, plus `plugins.updater` with the endpoint `https://updates.anima.ing/latest.json`, `allowDowngrades: true` and `requireSignedVersion: true`. | UPD-001, UPD-002, LOG-007, LOG-008, R-006, R-053 |
| Client | `$lib/updates/`:<br>- schedule: `unchecked → checking → downloading → installed`, plus `idle` and `failed`. It checks at start and every 4 h, installs without a prompt, stops once installed, and records a failure as one fixed log line with no dialog;<br>- `provideDesktopUpdates()`, which runs only in the desktop production app;<br>- `UpdateBanner` (a persistent `role="status"` region, "Anima <v> is installed", "Restart to update");<br>- `UpdatesRow` and `DesktopRows`.<br>`$lib/process-log/LogFolderRow`. Adapters in `$lib/desktop/updater.ts` and `processLog.ts` (R-056). | UPD-001–006, LOG-008, SET-002 |
| Docs | `docs/releases.md`: how a release happens; operator steps 0 and a–i; how the key and secrets are held; what protects the feed; known issue; rollback; failed runs | UPD-012, DNS-007 |

## Checks

- `bun run gates` on the final tip: see the last line of this report.
- Local packaged run (T4):
  - The release `tauri build` of `Anima.app` (21.36 MiB) used a throwaway config: no updater artifacts, and a feed address that refuses connections.
  - The app checked at start and logged the updater error plus the client's fixed line "Update check failed." to `~/Library/Logs/ing.anima.desktop/Anima.log`.
  - It kept running with no dialog.
  - I did not exercise by hand: the banner and restart (they need a signed feed) and opening the folder in Finder (unit-tested with stand-in openers).
- **Not run on GitHub.** The workflow cannot run without a push. These are unverified until the first run:
  - the runner package list;
  - the artifact paths;
  - `tauri bundle` on Linux with `--no-sign`;
  - the version bound into the macOS signature. `make-feed`'s signed-version check fails the run loudly if it is missing.

## Operator steps (UPD-012, DNS-007; agents never do these)

The full text, with commands, is in `docs/releases.md` "Once, by the operator". In order:

0. **Branch.** The default branch is `claude/extensions-planning-c47471`, not `main`. The workflow fires only once `main` exists and holds `release.yml`.
1. 🔒 **`tauri signer generate`** (file `~/.tauri/anima-updater.key`). Run `bun run --cwd apps/desktop tauri signer generate -w ~/.tauri/anima-updater.key`, then `chmod 600 ~/.tauri/anima-updater.key`. Do not use bunx or npx: `bunx tauri` fetches an unrelated 2020 npm package. Back up the key and its password offline (a password-manager attachment and an encrypted USB copy). If they are lost, every installed app must be reinstalled by hand.
2. **Public key.** Paste the contents of `~/.tauri/anima-updater.key.pub` into `apps/desktop/src-tauri/tauri.conf.json` `plugins.updater.pubkey`, replacing `REPLACE_WITH_OPERATOR_PUBLIC_KEY`. Commit it through phase-1. The workflow refuses to build while the placeholder is there.
3. **`release` environment.** Create it with a deployment branch policy of `main` only, before `main` exists. GitHub otherwise creates it unprotected on first use.
4. 🔒 **`gh secret set`** (file `~/.tauri/anima-updater.key`):
   - `gh secret set --env release TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/anima-updater.key`
   - `gh secret set --env release TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, only if you set a password; it prompts.

   These are environment secrets, not repository secrets.
5. **Cloudflare (DNS-007).**
   - Create a proxied `updates` record.
   - Add one Redirect Rule, 302, for the four paths `/latest.json`, `/Anima.dmg`, `/Anima-x86_64.AppImage` and `/SHA256SUMS`, sending each to `https://github.com/TheArctesian/anima/releases/latest/download/<name>`.
   - Turn the query string off and URL normalisation on. Every hop is https.
   - Check with `curl -sI https://updates.anima.ing/latest.json`.
6. 🔒 **Optional:** all six Apple secrets, for Developer ID signing and notarization.
7. **Hardening:**
   - immutable releases;
   - 2FA on GitHub and Cloudflare;
   - SHA pinning required for actions;
   - `gh cache delete --all` if any run happened before the first release.
8. **First release, then acceptance item 16:**
   - Create `main` from a phase-1 commit whose gates pass, watch the run, and check the feed.
   - Install the first build by hand. An ad-hoc app needs "Open Anyway" or `xattr -dr com.apple.quarantine`.
   - Merge a second change, see the banner, use "Restart to update", and check the version in Settings.
   - Test a bad signature only against a local throwaway feed, never a real release.

## Residual risks (recorded, accepted)

- **The macOS key step and the build share one runner**, so build-time dependencies (npm, cargo) are trusted as much as the key. Developer ID signing with an external signer would reduce this later.
- **`allowDowngrades: true` (R-006).** Anyone who controls `updates.anima.ing`, the Cloudflare account or the GitHub release can roll apps back to any older signed release. `requireSignedVersion` stops only mislabelled versions.
- **Ad-hoc signing** probably makes macOS ask for Keychain access to the stored token again after each update.
- **The web view can pass `headers`, `timeout`, `proxy` and `target` to the updater's `check`, and can relaunch the app.** It cannot change the key, the endpoints or the downgrade rule.

## Edits outside the owned paths (R-006, R-043, R-053, R-056)

- **WP-11** `routes/+layout.svelte`: +4 lines (2 imports, `provideDesktopUpdates()`, `<UpdateBanner />`). That is a deviation from "one mount line", because the Settings rows need the context the call provides.
- **WP-15** `routes/settings/+page.svelte`: +1 import, plus `<DesktopRows />` at the marked place. Its `page.test.ts` mock gains `keyStoreAvailable`.
- **WP-16** files:
  - the plugin lines in `Cargo.toml`, plus `serde_json` moved to normal dependencies (needed by `generate_context!`);
  - `main.rs` registration;
  - the command entry in `build.rs` (needed for the permission);
  - the two `tauri.conf.json` keys;
  - two new files in `$lib/desktop/` (R-056).
- Root `Cargo.toml` workspace dependencies and `Cargo.lock` (23 new crates, nothing upgraded); `apps/client/package.json` (three exact plugin versions); `bun.lock`; `deploy/tsconfig.json` (`release` included).

## Handoffs

- **H-1 WP-01:** add release-cfg clippy (`cargo clippy -p anima-desktop --release --all-targets -- -D warnings`) to the gates. The updater registration is compiled only in release.
- **H-2 WP-15 (could):** add a disabled style for "Sign out".
- **H-3 spec:** amend SET-002 to include the "Open log folder" row (R-053, LOG-008).
- **H-4 operator:** the steps above, then acceptance item 16.

## Not built

- No "Check for Updates…" menu item. The shell has no menu, and UPD-006 puts the button in Settings (D-6).
- No iOS or web updater, by design (UPD-001).

**Gates:** `bun run gates` on `bf111b9`: **12/12 pass**. Test counts: client 1178, deploy 300, headscale 433, design 117, quality 69, dns 29, desktop shell 30, hub 773. `DATABASE_URL` was unset.
