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
