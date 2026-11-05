# WP-04 stage B — Desktop releases and self-update (package plan)

| Item | Value |
| --- | --- |
| Package | WP-04 Deployment, stage B (R-006), plus LOG-008 (R-053) |
| Package branch | `claude/wp-04b-releases` (from `phase-1` at `9a4623e`) |
| Task branches | `wp/04b-releases/<task-slug>` (COM-003) |
| Owned paths | `.github/workflows/release.yml`, `deploy/release/`, `deploy/test/release-*.test.ts`, `apps/client/src/lib/updates/`, `apps/client/src/lib/process-log/`, `apps/desktop/src-tauri/capabilities/desktop.json`, `docs/releases.md` |
| Edits in accepted packages (R-006, R-043, R-053; one at a time) | WP-16: plugin lines in `apps/desktop/src-tauri/Cargo.toml` and `src/main.rs` (the crate has no `lib.rs`), `bundle.createUpdaterArtifacts` and `plugins.updater` in `tauri.conf.json`, plugin workspace dependencies in the root `Cargo.toml` and `Cargo.lock`. WP-11: one banner mount line in `routes/+layout.svelte`. WP-15: the Updates row and the "Open log folder" row at the marked place in `routes/settings/+page.svelte`. P-1 below: new files in `$lib/desktop/`. |
| Spec | UPD-001–012, LOG-008, DNS-007 (operator steps only), SET-002, LOG-007, CQ-010 / R-005 |
| Depends on | WP-15, WP-16 (accepted) |
| Models | Builders `builder` (Sonnet), `builder-heavy` (Opus) for the shell updater; reviewers `reviewer` on every task, `reviewer-security` (Opus) on T1, T2 and the final review, `reviewer-interface` on T3 |

## Decisions

- **D-1 Release workflow (UPD-007–011, R-005).** `.github/workflows/release.yml` is the only workflow. Triggers: `push` to `main` and `workflow_dispatch`; no `pull_request`. `concurrency: { group: release, cancel-in-progress: false }`. Top-level `permissions: contents: read`; only the publish job gets `contents: write`. Every third-party action is pinned by commit SHA. Jobs:
  - `version`: `0.1.${{ github.run_number }}` and the tag `v0.1.<n>`; nothing is committed.
  - `build-macos` (`macos-15`): `tauri build --target universal-apple-darwin` with the version passed through `--config`; ad-hoc signing (`APPLE_SIGNING_IDENTITY=-`) unless every Apple secret is present, then Developer ID signing and notarization; `codesign --verify --deep --strict` and `lipo -archs` = `x86_64 arm64` (UPD-011). Uploads the `.app.tar.gz`, its `.sig` and the DMG as job artifacts.
  - `build-linux` (`ubuntu-24.04`, WP-16 H-2 packages): `tauri build --bundles appimage`; uploads the `.AppImage` and its `.sig`.
  - `publish` (`if: always()`, the version job passed and at least one build passed): runs the `deploy/release/` scripts to make `latest.json` from the platforms whose build job **succeeded in this run** (UPD-009), the stable names `Anima.dmg` and `Anima-x86_64.AppImage`, and `SHA256SUMS` (UPD-010); creates the release as a **draft**, uploads every file, `latest.json` last, then publishes it and marks it latest (UPD-008).
  - Fails early when `plugins.updater.pubkey` is still the placeholder or `TAURI_SIGNING_PRIVATE_KEY` is unset. The key reaches only the build step, as an environment variable, and is never echoed (UPD-012, LOG-007).
- **D-2 Feed (UPD-009, DNS-007).** `latest.json`: `version`, `pub_date`, `notes`, and `platforms` with `darwin-aarch64` and `darwin-x86_64` (both the universal `.app.tar.gz`) and `linux-x86_64`, each `{ signature, url }`. URLs are the versioned files of this release (`https://github.com/TheArctesian/anima/releases/download/v0.1.<n>/<file>`), so marking an older release latest rolls back to its own files. The app's endpoint is `https://updates.anima.ing/latest.json` (DNS-007, the operator's Cloudflare redirect; the report has the steps). The feed logic is a pure TypeScript module in `deploy/release/` with a thin CLI, tested in the `deploy` workspace.
- **D-3 Shell (UPD-001, UPD-002, LOG-008, REPO-003).** `tauri-plugin-updater`, `tauri-plugin-process` and `tauri-plugin-log` are workspace dependencies. The updater and process plugins are registered only in release desktop builds (`#[cfg(all(desktop, not(debug_assertions)))]`), so `tauri dev` never checks (UPD-001). The log plugin writes to the OS log folder (`LogDir`: macOS `~/Library/Logs/ing.anima.desktop`, Linux `$XDG_DATA_HOME/ing.anima.desktop/logs`) with rotation, at `info`. One shell command, `process_log_folder_open`, opens that folder and nothing else; no opener or shell permission is granted to the web view. `tauri.conf.json`: `bundle.createUpdaterArtifacts: true`; `plugins.updater`: `pubkey` (a placeholder until the operator's step), `endpoints: ["https://updates.anima.ing/latest.json"]`. `capabilities/desktop.json` grants the `main` window only `updater:allow-check`, `updater:allow-download-and-install`, `process:allow-restart`, `log:allow-log` and the log-folder command.
- **D-4 Client (UPD-001–006, LOG-008).** `$lib/updates/` holds a pure update state machine (`idle → checking → downloading → installed`, plus `failed`), driven by an injected clock and an injected updater port:
  - it checks at start and every 4 hours (UPD-001);
  - it downloads and installs without a prompt (UPD-003);
  - it stops checking once an update is installed (UPD-005);
  - a failed check or a bad signature records a fixed-text log line and waits for the next check, with no dialog (UPD-002, UPD-006);
  - `allowDowngrades: true` (R-006).

  `UpdateBanner.svelte` ("Anima <version> is installed", "Restart to update", `role="status"`, UPD-004) mounts once in the root layout. `UpdatesRow.svelte` (current version, state, "Check for updates", UPD-006) and `process-log/LogFolderRow.svelte` ("Open log folder", LOG-008) mount on the Settings page only when `keyStoreAvailable()` (desktop). The web and iOS clients never start the schedule (UPD-001). Motion goes through `$lib/motion/`; values come from tokens.
- **D-5 Keys (UPD-012).** Agents never run `tauri signer generate`, never read a private key file and never set a secret. The operator:
  - makes the key pair and keeps a backup;
  - pastes the public key into `plugins.updater.pubkey`;
  - sets `TAURI_SIGNING_PRIVATE_KEY` (and its password, if any) with `gh secret set`.

  The report lists the steps.
- **D-6 Scope.** There is no "Check for Updates…" menu item: the shell has no menu, and UPD-006 puts the button on the Settings page. The iOS and web clients are not built in Phase 1.

## Proposals

- **P-1 (accepted as R-056).** The Tauri JS API may be imported only in `$lib/desktop/` (WP-16's folder, R-043 P-3). Stage B adds two new files there and changes nothing else in that folder:
  - `updater.ts`: the updater and process plugin calls, behind the D-4 port;
  - `processLog.ts`: `log` and `process_log_folder_open`.

## Tasks

| Task | Content | Files | Spec | Builder | Reviewers | After |
| --- | --- | --- | --- | --- | --- | --- |
| T1 `release-workflow` | D-1, D-2; tests for the feed, sums and stable names, and for the workflow's triggers, concurrency, permissions and job order (parsed YAML); `docs/releases.md` | `.github/workflows/release.yml`, `deploy/release/`, `deploy/test/release-*.test.ts`, `docs/releases.md` | UPD-007–012, DNS-007 | builder | reviewer, reviewer-security | — |
| T2 `shell-updater` | D-3; Rust tests for the log-folder command | `apps/desktop/src-tauri/` (R-006 lines), `capabilities/desktop.json`, root `Cargo.toml` dependencies, `Cargo.lock` | UPD-001, UPD-002, LOG-008, REPO-003 | builder-heavy | reviewer-security | — |
| T3 `client-updates` | D-4; the P-1 files; the banner and the rows | `apps/client/src/lib/updates/`, `apps/client/src/lib/process-log/`, the P-1 files, the layout and Settings lines, the plugin packages in `apps/client/package.json`, `bun.lock` | UPD-001–006, LOG-008, SET-002 | builder | reviewer, reviewer-interface | T2 names (fixed in D-3) |
| T4 `package` | A local `tauri build` of the macOS app with the updater on and a throwaway config (no key): the plugin loads, a failed feed is quiet, the log folder fills and opens | none (report) | UPD-001, UPD-006, LOG-008 | orchestrator | — | T2, T3 |

T1 and T2 run in parallel; T3 runs after T2. Final package review: `reviewer-security` on the whole diff against `phase-1`.

## Acceptance (operator, after merge to `main`)

Spec acceptance item 16:
- A merge to `main` publishes a release.
- An installed earlier build downloads it, shows the banner, and runs the new version after "Restart to update".
- A feed with a bad signature is refused, and the app keeps running.

This needs the operator's key, the secret and the Cloudflare redirect, so it runs after the report.

## Review log

(filled in as tasks run)
