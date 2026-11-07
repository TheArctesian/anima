# Phase 2 coulds for issue #1

These items are not blocking. The programme orchestrator collects them here and posts them to GitHub issue #1 in batches.

## WP-23 (gateway, compose and VPN policy)

- [ ] The Dockerfile is checked only as text.
- [ ] Images are pinned by tag, not by digest.
- [ ] The Windows machine needs its own RDP grant once it has a name and an address.
- [ ] `deploy/README.md` needs the two new variables.
- [ ] The SSH session target check does not read `ANIMA_PROBE_DENY` yet.

## WP-24 (client RDP and VNC tabs)

- [ ] The `if (!closed)` guard in `read()` in `view/certificate/CertificateModal.svelte` has no test.
- [ ] The `target === client` and `rules.paste` guards are redundant. They survive mutation.

## WP-25 (client device form)

- [ ] Phase 1 patterns to fix everywhere in one change:
  - 13 px checkboxes (UI-013)
  - hover contrast of 1.17:1 under more contrast (MAT-006)
  - primary button contrast (UI-004)
  - the press role animates background colour (MOT-006)
- [ ] Comments and test titles cite decision IDs (COM-002).
- [ ] The port and URL checks are duplicated.
- [ ] `sameJson` depends on key order.
- [ ] Changing an SNMP check's destination gives no warning.
- [ ] Saving an unchanged web interface row gives no feedback.
- [ ] `hub-pending.ts` still holds the device and web interface read types. This waits on `lib/devices/api.ts`.

## WP-20 (skeleton and contract gate)

- [ ] `host_port` accepts `[]:4822` and port 0.
- [ ] The fixture test does not check field names.
- [ ] The contract test has duplicated code.
- [ ] A blank line splits a table in `hub-domains.md`.
- [ ] The MV-008 allow-list does not exist.

## WP-30 (hub web interfaces)

- [ ] The T1 security mutation run hit its 20-minute limit and returned no results. It covered the URL rule, fingerprint parse, origin and pin drop, and the suggestion filter.
- [ ] Web interface events have device `None`. Filling it needs an import from `devices`.
- [ ] Add an `AuditTarget::WebInterface` kind. Today the entries target the device (H-3).
- [ ] Accepting or dismissing a suggestion reads the check outside the caller's transaction. Swap to `checks::http_urls` (H-1, WP-29).

## Programme

- [ ] Three tests fail under load:
  - `k4_8_5` (WP-22 is fixing it);
  - the deploy test DNS-003, for the `make-secrets` interrupt timing;
  - `checks::api::batching::api_005_status_last_heartbeat…`. With the 10 ms loader window under load, the run took 24 statements against a limit of 20, and it passed 3/3 alone. WP-29, which owns checks, should give the test schema a longer batch window.

## WP-21 (hub remote access settings)

- [ ] Clearing a wake relay automatically writes no audit entry of its own.
- [ ] A cleared relay's event has no device link.
- [ ] `devices/graphql/standing/interfaces.rs` is a name a newcomer would not look for.
- [ ] Nothing checks that `metricsSource: SSH` has SSH turned on.

## WP-24 (client RDP and VNC tabs, final)

- [ ] Microphone input and the shared drive in the client.
- [ ] The `onclient` test seam in `RemoteView`.
- [ ] Two exports that only tests use.
- [ ] No test cites RA-030.
- [ ] Check in the end-to-end run whether WebKit in the macOS app refuses device-to-system clipboard writes, because the write does not follow a user action.

## WP-31 (desktop web view)

- Add a test that a certificate challenge for a closed tab is cancelled.
- Add a load-failure hook, so an unreachable device shows the error page at once instead of after the 30 s timeout.
- Emit an event when the operator closes a popped-out window.
- Revisit the navigation filter, which refuses `about:`, `data:` and `blob:` frames that some device pages may need.
- Move the system trust check for tabs with no pin off the main thread.
- Run the shell commands in tests.
- Skipped: the separate whole-package final review (Section 3.2 step 4). Each task had an Opus security review, and T5 also had a Sonnet interface review.
- Operator: run the OD-7 spike on Linux (`apps/desktop/src-tauri/spikes/od7`, `xvfb-run -a cargo run`). Try a real device with an untrusted certificate (first visit, accept, changed certificate).

## WP-27 (SSH link, SFTP, wake on LAN)

- Add a DNS-rebinding test and a test for the per-address bound. There is no cheap loopback blackhole for the second.
- If a create is still pending past `CLOSE_WAIT`, the file can stay on the device.
- `russh-sftp` decodes non-UTF-8 file names lossily.
- A single directory listing can reach about 60 MB.
- `MAX_PENDING` for files tickets is 32 across the whole hub, not per operator.
- `cancel` is ignored while the file is still opening.
- The `TimeLimit` sentence says "10 seconds" twice.
- The host-key-changed wording differs between wake, files and the terminal.
- The terminal could open through the link.
- The link-error to GraphQL-code mapping is written in two places.
- Programme: add a `clippy.toml` `disallowed-methods` rule banning `Box::leak`, `String::leak` and `Vec::leak` (CHK-012).

## WP-28 (hub metrics)

- GPU discovery and GPU metrics (MET-006 to MET-010) are not built. The tables and the `gpus` face exist (R-073).
- Narrow the metrics public surface to `pub(crate)` where nothing outside the crate uses it.
- Add a metrics test that a deny-listed device gets no samples.
- Rename the `met_005_*` subscription tests and fix one stale doc comment.
- Finish the guard and filter mutation run on `metricsAdded`.
- Process: one shared Cargo target directory across worktrees can leave stale test binaries.

## WP-33 (client detail view and charts)

- Add a K5 opacity token for the chart min-max band, which uses `fill-opacity: 0.2` today.

## WP-32 (client files, wake and live sessions)

- A download cancelled after the hub has read every byte, but before the client has received them all, shows as cancelled on the client and is audited as done on the hub.
- The files panel's busy guards are pinned only through the link's busy refusal.
- Downloads are held in memory as a Blob.
- The files panel has no compact-width layout.
- In the Tauri shell, wry saves a download under its suggested name in ~/Downloads. Check the save path in the app.
- Revoking the blob URL at once may fail a WebKit download. Check it in the app.
- WP-34: a failed Stop in the live-sessions list shifts the rows once (MOT-007).
- WP-34: check a real download inside the Tauri app.
- WP-34: add a test for the live-sessions popover's reduced-motion form.
- The ping and silence timer code is duplicated between the files link and the terminal link. Lift it into one shared file.
- File names that are not valid UTF-8 come through with their bad bytes replaced, so the client cannot send them back.
- The file panel shows that upload or download is turned off only after the hub refuses an attempt.
- Downloads have no size warning.

## WP-33 (final)

- Host uptime (`UPTIME_SECONDS`) is fetched but not shown. Add a fact row if wanted.
- The chart skeleton draws four cells, but a device can have 5 to 9 or more charts. The skeleton could reserve the last-known chart count.
- Time labels jump one step on a chart update while the line slides. A label cross-fade could be added.
- Weekday tick labels are fixed English. Use `Intl.DateTimeFormat`.
- When the clocks go back, an hourly tick in the repeated hour gets one label, not two.
- WP-34: confirm the D-2 reading of 5.7.3 "moves left continuously", which is one spring slide per point (MOT-011).
- Live points update only the 1-hour range.
- A stale hub `readings` answer can briefly replace a newer live reading during a load.
- Add a test for colour wrap-around past six series.
- Under `prefers-contrast: more`, the chart grid stays `chart-grid` instead of `text-muted` (MAT-006). WP-34 should check this.
- The Sessions Start and End timestamps wrap at 600 px.
- `MetricsSection.svelte:5` has a four-space indent on one import.
- H-2: show `Session.clientAddr` in the Sessions tab (`DeviceSessions`; D-10, RA-034).
- One shared `endTransitionsAtOnce` fixture under `lib/motion/`.
- `routes/devices/[id]/` is at the 12-file limit. The next file there goes into a subfolder.

## WP-29 (hub check types)

- Operator decision (should): the sealed SNMP secrets are bound to the check, the version and the user name, but not to the device host. Anyone who can write to the database and changes `devices.host` gets the secrets sent to the new host. D-5 accepted this risk. Binding the host fixes it, but then every operator host edit needs the secrets entered again.
- The client SNMP editor (WP-25) does not check three hub limits: a v3 password of at least 8 characters, a user name of at most 32 bytes, and an OID of at most 128 arcs (WP-29 H-1).
- "SSH off" and "no credential" record Down. Unknown would be truer.
- The link error is a free-form string, not a typed reason.
- The scheduler's catch-all match arm could list each check type.
- The two seal-and-open files repeat each other.
- Some hash temporaries are not wiped, and a malformed SNMP datagram ends the attempt instead of being dropped.
- There is no audit test for setting only a v2c community.
- Several checks files are near the 300-line limit.
- Live: Docker and systemd need a real SSH device; SNMP v3 needs a real SHA-256 and AES agent.

## WP-26 (client device list and header)

- Should (WEB-010): an open web tab keeps the device status from when it opened, so a device that goes Down mid-tab does not show the Down page.
- WP-34: button borders are 1.36:1 against the card. Check them against the spec's contrast rule for UI components.
- At a 1280 px window the device table is 1506 px wide, so the Connect column is off-screen until you scroll.
- There is no Wake button in the table row or the header.
- `DeviceActions` restyles `WakeButton` internals through `:global(.status)`.
- The warning wording exists twice.
- The DEV-001 200-card test bound is 60 s under load.
- A 1024 px window gets compact header rows.
- `lib/sessions/web/open/` sits inside WP-31's folder.

## WP-22 (hub RDP and VNC sessions)

- The password copy in `ArgValues` is not zeroed.
- A woken session has no cap of its own; only the ticket cap applies.
- The terminal timeout message is vague after a 120 s wake.
- The certificate-mode tests could be one table-driven test.
- The `vnc.rs` banner check has no property test.
- The lagged branch of the `sessionChanged` stream is untested.
- Old plan IDs remain in comments written before WP-22.
- Live: Windows RDP with NLA (resolution, clipboard, Ctrl+Alt+Delete, a changed certificate ending 4003); the `sha256:` format of `cert-fingerprints`, worked out from the FreeRDP source and not yet checked on a real server; VNC to macOS Screen Sharing and to a Linux desktop; wake through a real site relay; Hyper-V may fail closed with 503.
- The wake line's icon nudges 2 px sideways when it changes.
- The ended-panel button widens when its text changes between Reconnect and "Check certificate".
- WP-34: the ended text on a 4003 close may read "You ended the session." Check it against the real hub.

## WP-34 (acceptance fixes)

- F5 (should): the compact-header `767.98px` media query literal is not tied to the `bp-compact` token by a test; a token change would not fail any test.
- F5 (should): the narrow-header test runs in jsdom, which cannot evaluate the media query; a browser test should prove the wrap and icon-only badge at 390 px.
- F3 (could): in Chromium only, the settled close of the last session tab logs one layout-shift entry (0.0198) from the hidden `.stage`; nothing visible moves.
- F7 (should): at 390 px the Overview chart skeleton is 1382.5 px against 1432 px for the data, because the legend row wraps in one column; the area grows 49.5 px at the bottom (CLS 0, nothing below it).
- F7 (could): `rememberChartShape` then `chartShapeOf` round-trip through the shapes map (`routes/devices/[id]/overview/metrics.svelte.ts:79-82`); pass the shape directly.

## CQ-050-P2 (Phase 2 code-quality SPEC items)

- Non-SPEC audit items Q2-05 to Q2-16 in `docs/research/code-quality/phase-2.md` (R-076).
- T3 (could): `Detail.applyCheck` could be private; `pauseCheck` and `saveCheckValues` are tested only through `page.test.ts`; `DeviceMissing.svelte` carries an empty script block for svelte-check.
- T1 (could): the `token_half` mutant survives (`checks.rs:253-257`); `Vault::seal` draws its own nonce, so it cannot be passed in for tests (`credentials/vault.rs:68-72`); `rand::rng()` at `checks/scheduler.rs:48` still panics on a random-source failure (startup jitter only).
- T2 (should): in WebKit the opener focus return misses conditionally mounted callers (S1, as before).
- T2 (should): after opening, the host key surface grows 96 px and the credential surface 32 px in one frame (MV-004, MOT-007; as before).
- T2 (should): on `--surface-raised`, modal fields and the Cancel button share the dialog fill and are separated only by a 1.2:1 border (shell-wide, also AddDeviceModal; UI-001, MAT-001).
- T2 (could): the literal-opacity rule skips `stop-opacity`, `flood-opacity` and `style:` bindings; the "Save password" checkbox is 12 px (UI-013); prompt heights differ from before by -22 to +10 px from the shell gap; one stack-test mutant hangs instead of failing; the CredentialPrompt `.content` wrapper is unused.

## WP-35 (one-button screen sharing, R-079)

- Could: a 0.0197 layout shift in Chromium when the session strip appears late after setup; it belongs to the session shell.
- Could: the status panel is placed twice on mount.
- Could: no visible Cancel while testing (Escape works).
- Could: the "+ VNC" button is 57 px against 56 px for VNC.
- Could: `normalizeHost` would sit better in a shared hosts module.
- Operator: try the flow on this Mac with Screen Sharing off, then on, and on one Linux VNC device.
