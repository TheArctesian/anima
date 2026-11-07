# Phase 2 code-quality audit (CQ-050)

**No item in this report has been applied.** This is an audit-mode report. No source code was
changed (WP-34 D-3). Each item needs the operator's acceptance first. An accepted item is then
built on its own branch and pull request (CQ-050). It moves code without a rewrite (CQ-051). The
gates run, and the baseline is lowered where it applies.

Audit of `phase-2` code at `551ee1da` (branch `claude/wp-34-acceptance`), WP-34 T4. The auditor did
not build this code.

Some items below are also spec failures, not only quality items. They are marked **SPEC** in the
ID column. The operator may want to fix those under the acceptance criteria, not as refactors.

## 1. Method and scope

| Item | Value |
| --- | --- |
| Skill | `code-quality` (CUPID), audit mode. The skill file was read, but its multi-agent workflow was not available in this session (no `Skill` tool). One auditor ran the lanes one after another. There was no separate verifier vote. The auditor re-read the cited code before writing each item. No item was proved by a mutation run or a test run. |
| Rules | `AGENTS.md`; design spec Section 12 (CQ-001 to CQ-051) and Section 13 (REPO-001 to REPO-004). |
| Gates already passing | `bun run quality:limits`, `quality:names` and `quality:imports` print `ok` at this commit. `quality/baseline.json` is `{}`. So this audit does not repeat size, banned-name or import findings. It looks for what those gates cannot see. |
| Not run | `cargo build`, `cargo test` and `cargo clippy` (the machine was loaded). Claims about behaviour come from reading the code. An item that needs a run to prove it says so. |
| Scope | The Phase 2 diff `f3c6feb..551ee1da` (822 files). Hub: `sessions/`, `metrics/`, `checks/`, `web_interfaces/`, `devices/`, `credentials/`, `targets.rs`, `upkeep.rs`. Client: `lib/{charts,metrics,sessions,web-interfaces,devices,forms,desktop}` and `routes/devices/`. Tauri shell: `web_view/`. Deploy: `deploy/localapi/`. |
| Signals swept | Greps for `let _ =`, `unwrap`, `expect`, `unwrap_or_default`, `rand::`, `getrandom`, `Instant::now`, `Utc::now`, `catch {`, `as unknown as`, `eslint-disable`, `#[expect`, hard-coded colours and opacities, duplicate function names across domains (`fn internal`, `fn pool`, `fn begin`, `fn is_invisible`, `field_names`, `struct FieldError`, `struct Trail`, `struct Timing`), exports with no importer outside their file, and folder file counts. |
| Already on issue #1 | `docs/programme/phase-2-coulds.md` is not repeated. An item that adds evidence names the package it extends (for example "WP-32"). Items from Phase 1 that are still open are in Section 3, with their new status. |
| Ranking | By value to the operator, then by risk. A **SPEC** item comes first inside its group. Sizes: S is under half a day and one or two files; M is one to two days or several files; L is more. |

## 2. Ranked items

Every item is a branch and a pull request of its own (CQ-050). The suggested branch name is
`quality/q2-nn-<slug>`.

| ID | Property | Location and evidence | Proposed move (no rewrite) | Risk | Effort |
| --- | --- | --- | --- | --- | --- |
| **Q2-01 SPEC** | C | **The web interface section and four remote-access calls are built and tested but never mounted.** `routes/devices/[id]/settings/SettingsPanel.svelte:28,57` takes `webInterfaces` as an optional prop and shows the section only if it is set. `routes/devices/[id]/+page.svelte:244-253` does not pass it. So `lib/forms/device/web/WebInterfaces.svelte` has no production use. `webInterfacesApi` (`lib/web-interfaces/api.ts:78`) has no caller outside its own test. The device query does fetch `webInterfaceSuggestions` (`lib/devices/api.ts:171`) and nothing reads it. The route also passes none of `onClearRdpCredential`, `onClearVncCredential` and `onAcceptServerCertificate` (`DeviceForm.svelte:54-77`); `devices.clearRdpCredential` and `clearVncCredential` (`lib/devices/client.ts:81,86`) have no caller. `devicesApi` has no `setRdpCredential` or `setVncCredential`, so `saveDevice` (`routes/devices/[id]/save.ts:23-24,56,61`) turns a typed RDP or VNC password into a failure. `createWithCredential` in `routes/devices/+page.svelte:104-111` has the same gap (`lib/forms/add-device.ts:21-22`). `docs/programme/packages/WP-25-report.md` H-2 ("Pass `onClearRdpCredential`, `onClearVncCredential`, `onAcceptServerCertificate` and `webInterfaces`") was never done. Spec: WEB-001, WEB-002 (criterion 10, no operator route to accept a suggestion or add an interface), criteria 1 to 3 (stored RDP and VNC password, certificate accept from Settings), CQ-001 (dead exports). | This is a **fix task**, not a refactor. In the route, pass `webInterfaces` (`interfaces`, `suggestions`, `calls` from `webInterfacesApi(hub)`), the three remote callbacks, and give `saveDevice` and `createWithCredential` the `setRdpCredential` and `setVncCredential` calls that `sessionsApi.setCredential` already wraps (`lib/sessions/api.ts:236-243,285-288`). Move the new callbacks into `Detail` (see Q2-04), not into the route. Add one route test per call. | Low | M |
| **Q2-02 SPEC** | P | **Secret-making can panic on an OS error.** `credentials/vault.rs:71` is `XNonce::try_generate().expect("the OS random source must yield a nonce")`, and its doc says "# Panics" (`vault.rs:64-69`). `seal` is infallible, so every seal of an RDP or VNC password (`credentials.rs:188`), an SSH part (`credentials.rs:90`), an HTTP or SNMP secret (`checks/store/sealed.rs:105`, `checks/store/snmp_sealed.rs:104`) and a TOTP secret (`access.rs:285`) can panic on I/O. Also `rand::fill` at `checks/graphql/write.rs:44` (push token) and `checks/probe/snmp.rs:45` (SNMP ids and salt, Phase 2 new). Elsewhere the code does it right: `getrandom::fill(..).map_err(Error::Random)` at `access/login.rs:132`, `sessions/start.rs:180` and `sessions/files/opening.rs:71`. Spec: CQ-023 ("never use `expect` on I/O"), CQ-021 (randomness passed in). Extends Phase 1 CQ1-13. | Give `credentials::Error` a `Random` case and make `Vault::seal` return `Result<Sealed, Error>`. Every caller already returns a `Result` with a credentials case. Move `fresh_token` into the checks domain with `getrandom` and a `checks::Error::Random`. In the SNMP probe, pass `Draws` in, as the scheduler does. | Low to medium (a signature change across five call sites). | M |
| **Q2-03 SPEC** | I | **Three session modals rebuild the modal shell, and each has a literal scrim.** `lib/sessions/view/certificate/CertificateModal.svelte:187-191`, `hostkey/HostKeyModal.svelte:141-145` and `credentials/CredentialPrompt.svelte:155-159` each set `.shade { background: var(--bg); opacity: 0.72 }`. The value 0.72 is in no token. The scrim token is `--scrim` (`design/tokens/color.tokens.json:233-240`, "nord-0 at 55 % behind a modal task (MAT-003, R-049)"), and `lib/forms/modal/Modal.svelte:125-128` uses it. The three also import the second focus trap, `lib/sessions/view/hostkey/focus.ts` (`CertificateModal.svelte:7`, `CredentialPrompt.svelte:5`), while `Modal.svelte:5` uses `forms/modal/focus.ts`. Spec: MAT-003, R-049 and the token rule (K5, "design values come from tokens"). The token gate did not catch it, so it does not check literal `opacity`. Extends Phase 1 CQ1-08, which counted two traps and one modal; there are now four modals and two traps. | Make the three use `Modal.svelte` (or its scrim and trap), and delete `hostkey/focus.ts`. Add a literal-`opacity` check to the token gate. Keep each modal's own content and states. The motion owners must re-run MV-003 and MV-004 on the three, because the scrim fade and the focus return path change. | Medium (keyboard focus and motion). Interface review needed. | M |
| **Q2-04 SPEC** | U | **The device detail route holds logic.** `routes/devices/[id]/+page.svelte` has the tab-start effect (lines 73-82, `started`), the metrics-key effect (84-97, `metricsKey`), and six mutation wrappers (`saveSettings`, `acceptHostKey`, `clearCredential`, `deleteDevice`, `putCheck`, `dropCheck`, lines 121-161). Spec: CQ-044 ("a route composes components; it does not contain logic"). `id ?? ''` appears at lines 52, 53, 130, 136 and 145, although `+page.ts:5` makes `id` null for a bad id, so the page then holds a `metrics` and a `sessionPages` object that would ask the hub for an empty id. Today no path reaches them, because `device` stays null, but nothing in the types says so. Phase 1 CQ1-09 named the same code; it has grown (metrics, sessions and logs were added). | Move the six wrappers and the two effects into `Detail` (`detail.svelte.ts`) as methods that take the domain APIs. Build `Detail`, `DeviceMetrics`, `Pages` and `LogFeed` only for a non-null id (a child component `DetailBody` that takes `id: string`), so the `?? ''` goes away. The folder is at the 12-file limit, so the new file goes in a subfolder. | Low to medium (the route tests cover the flows). | M |
| Q2-05 | D | **`sessions/` is five domains in one folder (174 files).** It holds the session record and tickets (`sessions.rs`, `store.rs`, `tickets.rs`, `live.rs`), the SSH client (`ssh.rs`, `ssh/`, `link.rs`, `link/`, `probe.rs`, `public_key.rs`), host-key and certificate trust (`trust.rs`, `trust/`, 12 files), three tunnels (`terminal/`, `tunnel/`, `files/`) and wake (`wake/`). `checks` and `metrics` depend on `sessions` for the SSH link only: `checks/probe/ssh.rs:19,55`, `checks/probe/docker.rs:13`, `checks/probe/systemd.rs:11`, `metrics/collector/collection.rs:23,104`, `metrics/commands.rs:10`, `metrics/collector/outputs.rs:18`. That is two `quality/hub-modules.json` edges (`checks -> sessions`, `metrics -> sessions`) for a transport, not for sessions. `metrics` and `checks` must also import `sessions` for `FixedCommand` and `CommandOutput`. | **Move** `ssh`, `link`, `probe` and `public_key` to a new domain `ssh/` (the SSH client: link, fixed commands, host key). **Move** `trust/` to its own domain (`trust/`). `sessions` keeps record, tickets, live and the three tunnels. Fix the `hub-modules.json` edges: `checks -> ssh`, `metrics -> ssh`. Do it in two pull requests, `ssh/` first. Phase 1 said no split until RDP arrived. RDP and files have arrived. | Medium (many `use` lines; the import gate checks the result). | L |
| Q2-06 | P | **The K7 error plumbing grew from five copies to seven.** `internal`, `INTERNAL_MESSAGE`, `schema_dependency` and `UnregisteredDependency` are now in `checks/graphql/error.rs`, `devices/graphql/error.rs`, `maintenance/graphql/error.rs`, `sessions/graphql/error.rs`, `events/graphql.rs:190-206`, **`web_interfaces/graphql/error.rs:7-50`** and **`metrics/graphql.rs:42-58`**. The metrics comment says why: "the shape of the checks helpers, which this domain may not import". `pool`, `begin` and `commit` are copied in `devices/graphql.rs:132-144`, `maintenance/graphql.rs:23-35`, `web_interfaces/graphql.rs:23-35` and `checks/graphql/write.rs:22-34`. `field_names` is copied five times (`maintenance.rs:36`, `checks.rs:87`, `sessions.rs:142`, `devices.rs:57`, `web_interfaces.rs:50`), and `FieldError` is five identical structs (`checks/model.rs:168`, `sessions/model.rs:221`, `maintenance/rules.rs:26`, `devices/rules.rs:26`, `web_interfaces/model.rs:110`). The K7 codes are bare literals in `sessions/wake/graphql.rs:91-108` and `sessions/files/protocol.rs:139`, and the process log matches on them (`events/process_log/internal_errors.rs:23`). Extends Phase 1 CQ1-01 and CQ1-14. | As CQ1-01: one leaf `crates/hub/src/api_errors.rs` with a typed `ErrorCode` enum, `internal`, `coded`, `invalid`, `schema_dependency`, `FieldError` and `field_names`, and generic `pool`, `begin`, `commit`. Add it to `hub-modules.json` `leaves`. Each domain keeps its own `to_graphql(Error)`. The metrics domain then reuses it, and the "may not import" comment goes. | Low to medium. The wire codes must not change; the K7 tests pin them. | M |
| Q2-07 | U | **The audit `Trail` is now written five times.** `checks/graphql/trail.rs:38-60`, `devices/graphql/trail.rs:41-60`, `maintenance/graphql/trail.rs:33-50`, **`web_interfaces/graphql/trail.rs:36-60`** and, in another shape, `sessions/files/trail.rs:25`. Each has the same `from_context` (bus, operator, client address, clock read once) and the same write order (audit, then event). The WP-30 could "web interface events have device `None`" is a symptom: the trail takes the device from its own domain. Extends Phase 1 CQ1-04. | As CQ1-04: `events::Trail::write(conn, entry)` with the source on the entry. Each domain keeps its pure `Entry` builders. Do the web interface `device` field in the same pull request, since `Entry` then carries it. | Low (one transaction, the same order). | M |
| Q2-08 | C | **Three session relays copy one skeleton and one timer model.** `sessions/terminal/session.rs:37-72` and `sessions/tunnel/relaying.rs:46-81` differ only in the ticket check, the opening call and the message text (a diff shows about 20 lines of difference in 45). Three `Timing` structs (`terminal/route.rs:58`, `tunnel/route.rs:63`, `files/route.rs:65`) and three hand-written "restart the quiet timer after each turn" blocks (`terminal/link.rs:55`, `tunnel/relaying/link.rs:134-143`, `files/relay.rs:145`). The close writes are copied: `terminal/writes.rs:53-66` against `tunnel/relaying/writes.rs:66-77`. The WP-32 could names the same duplication on the client side; this is the hub side. Extends Phase 1 CQ1-10. | **Move** the shared skeleton (redeem, check the subprotocol, `handed_over`, `finish`) and the race-send and close writes into `sessions/upgrade.rs`, which already holds `handed_over`. Add one `Quiet` timer type there (`restart(after)`, `due()`). Each tunnel passes its `open` and its messages. | High (session tunnels; security review). | M |
| Q2-09 | P | **Two rules say which addresses a session may reach.** `sessions/target.rs:7-65` (`is_refused_target`, a second `embedded_v4` at line 24, `six_to_four_v4`, `is_refused_tunnel`) and `targets.rs:60-120` (`AddressPolicy`). `TargetRule::refuses` (`sessions/target.rs:117-121`) ORs them. By reading, every address `is_refused_target` refuses is also refused by `AddressPolicy::permits`: loopback, link-local, multicast, broadcast (covered by `o[0] >= 240`), `0/8`, unspecified, Teredo, local-use NAT64, and the embedded and 6to4 forms (covered by `canonical`). The one live difference is the test-only loopback switch (`loopback` in `TargetRule`, `allow_loopback` in the policy, kept equal by `aligned`, lines 108-118). So in a release build the sessions rule cannot change an outcome, and two copies of the IPv4-in-IPv6 decoder can drift. This is the "predictable" risk Phase 1 CQ1-06 named; the leaf `targets.rs` is half the fix. **To prove it, run one mutation: replace `is_refused_target(ip)` with `false` and run the sessions target tests. If nothing fails, the rule is dead.** | **Delete** `is_refused_target` and its helpers, and keep `TargetRule` as the loopback test seam plus the policy. Keep any case the test run shows the policy misses, and move it into `targets.rs`. Do it with the WP-23 could "the SSH session target check does not read `ANIMA_PROBE_DENY` yet". | High (SSRF boundary; security review). | S to M |
| Q2-10 | D | **Two SHA-256 certificate fingerprint types with two formats, plus a third name.** `sessions/trust/certificate.rs:15-61` (`CertificateFingerprint`, 32 hex pairs joined by colons, as FreeRDP writes it) and `web_interfaces/model.rs:35-52` with `web_interfaces/rules.rs:119-138` (`Fingerprint`, 64 lower-case hex digits, no colons). SSH has `sessions/public_key.rs:130` (`Fingerprint`, `SHA256:` and base64). The Tauri shell has `web_view/trust/fingerprint.rs` and the client `lib/desktop/webView/fingerprint.ts` (64 hex), and the client shows web fingerprints as upper-case colon pairs (`displayFingerprint`) while RDP ones come in lower-case colon pairs. AGENTS.md names `Fingerprint` as one domain noun (CQ-031). An operator copies a fingerprint from one screen to another, and the two forms differ. | **Move** the certificate digest to one leaf type `CertificateFingerprint` (`targets.rs` style, `crates/hub/src/fingerprint.rs`) that parses both forms and keeps one canonical value, with two renderings. The columns keep their own check constraints, so there is no migration. Keep the SSH `Fingerprint` in `sessions` (a different thing). In the client, one `displayFingerprint` for both. | Medium (a leaf type, and two stores keep their format). | M |
| Q2-11 | P | **Five variants of "invisible text" drift.** `is_invisible` is copied three times with different tables: `checks/probe/outcome.rs:12`, `sessions/trust/certificate.rs:105` (adds U+0890, U+08E2 and astral ranges) and `web_interfaces/rules.rs:111` (starts at U+202A, so **U+2028 and U+2029 pass**). `devices/text.rs:19`, `maintenance/targets.rs:20` and `checks/rules.rs:44` have their own `is_control() ||` lists. Result by reading: a web interface label with a line or paragraph separator is accepted (`web_interfaces/rules.rs:105`, `is_unsafe_in_label`), while the same character is stripped from a check message and a certificate subject. No test pins the difference. Adds evidence to the WP-09 item on issue #1 ("shared text-rule leaf"). | **Move** one `text.rs` leaf with `is_invisible` (the widest table, the sessions one) and `is_plain(&str)`. Each domain calls it. Add one test per class (bidi, zero width, line separator, tag). | Low. The web label rule becomes stricter; say so in the pull request. | M |
| Q2-12 | D | **Client files sit by layer or by transport, not by operator noun.** Phase 1 CQ1-07, CQ1-11 and CQ1-18 are all still open (`lib/forms/check/`, `forms/device/`, `forms/AddDeviceModal.svelte`, `forms/this-machine.ts`; camelCase `lib/desktop/thisMachine.ts`, `processLog.ts`, `systemSettings.ts`). Phase 2 added more: the web tab UI is in `lib/sessions/web/` (12 files, plus `web/open/`) and its Tauri bridge in `lib/desktop/webView/`, while `lib/web-interfaces/` holds one `api.ts`. Twenty client folders now sit at 11 or 12 files, among them: `lib/desktop` (12), `lib/forms/modal` (12), `lib/forms/check` (12), `lib/sessions/web` (12), `lib/sessions/view` (12), `lib/events/panel` (12), `lib/devices/card` (12), `routes/devices/[id]` (12), and `lib/metrics`, `lib/sessions`, `lib/nav`, `lib/hub` (11). The 12-file gate passes, but every next file forces a move. Also `lib/hub/hub.ts:8-15` is a barrel, yet 85 files import `$lib/hub/hub` and 50 import `$lib/hub/error` (Phase 1 CQ1-17 counted 54 and 35). | One move-only set, in this order: the Phase 1 moves (CQ1-07, 11, 18), then `lib/sessions/web/` and `lib/desktop/webView/` into `lib/web-interfaces/` (`tab/`, `shell/`). Rename the camelCase files. Add the casing rule to `quality:names`. Do the hub import face last, with an `eslint` restriction. | Low (moves only). | L |
| Q2-13 | I | **The Tauri shell runs an OS program four ways.** Phase 1 CQ1-12 counted three. The fourth is `system_browser.rs:73-98` (polls `try_wait`, then reaps in a new thread). The others: `process_log/folder.rs:36`, `this_machine/name.rs:51`, and `system_settings.rs:51-56`, whose `.status()` has **no wait limit** (`folder.rs:18-20` explains why openers need one). `system_browser.rs` and `system_settings.rs` both name their function `open_with` and set up `Command` the same way. | **Merge** into one shell module `os_program.rs`: run a program with a wait limit and a reap rule. `system_settings` then gets a limit. | Low. | S |
| Q2-14 | P | **Silent fallbacks when sealing a check.** `serde_json::to_vec(&Binding::of(config)).unwrap_or_default()` is at `checks/store/sealed.rs:103` and, new in Phase 2, `checks/store/snmp_sealed.rs:102`. `origin(&config.url).unwrap_or_default()` is at `sealed.rs:43`. A failure seals an empty binding and says nothing. It fails closed later (`SecretsUnbound` at probe time), so the operator sees the fault long after the save. Spec: CQ-024 (do not swallow an error), CQ-023. The WP-29 could "the two seal-and-open files repeat each other" is the same pair of files. Extends Phase 1 CQ1-15. | Use `expect` with the invariant ("plain strings and bools always serialise", CQ-023). Make `seal` return `Result` when the URL has no origin, so the save fails with `Validation`. Fold into the same pull request that merges the two files, if WP-29's could is accepted. | Low. | S |
| Q2-15 | P | **Unchecked casts at a parse boundary.** `deploy/localapi/allow.ts:17`: `parts.map(Number) as [number, number, number, number]`. The earlier line checks the length, but the type does not say so. Phase 1 CQ1-20 found the same pattern at `quality/src/hub-modules.ts:46` and `deploy/dns/address.ts:32`, and both are still there. The Phase 2 client casts are all guarded (`lib/sessions/tunnel/terminal/codec.ts:61`, `lib/desktop/webView/fingerprint.ts:10`). | Build the tuple from four checked numbers in `allow.ts`. Do the two Phase 1 sites in the same pull request, with the `isRecord` guard from `quality/src/baseline.ts:41`. | Low. | S |
| Q2-16 | D | **Two things called "charts".** `lib/charts/` holds the drawing primitives (`LineChart`, `plot`, `scales`, `ticks`). `lib/metrics/charts.ts` turns metric series into chart models. Each has a `charts.test.ts`. `lib/metrics/charts.ts:4` also imports `DEGRADED_ABOVE` from `./bars/bars`, so the chart threshold lives in the bar file. | **Rename** `lib/metrics/charts.ts` to `lib/metrics/overview-charts.ts` (it builds the Overview's charts, D-4). Move `DEGRADED_ABOVE` to `lib/metrics/units.ts` or a `thresholds.ts`. | Low. | S |

### Counts

| Property | Items |
| --- | --- |
| C (Composable) | 2: Q2-01, 08 |
| U (Unix philosophy) | 2: Q2-04, 07 |
| P (Predictable) | 6: Q2-02, 06, 09, 11, 14, 15 |
| I (Idiomatic) | 2: Q2-03, 13 |
| D (Domain-based) | 4: Q2-05, 10, 12, 16 |

| Lane | Items |
| --- | --- |
| Hub | 9: Q2-02, 05, 06, 07, 08, 09, 10, 11, 14 (Q2-10 also touches the client) |
| Client | 6: Q2-01, 03, 04, 10, 12, 16 |
| Tauri shell | 1: Q2-13 |
| `deploy/` and `quality/` TypeScript | 1: Q2-15 |

| Class | Items |
| --- | --- |
| Also a spec failure (SPEC) | 4: Q2-01 (WEB-001, WEB-002, criteria 1 to 3 and 10), Q2-02 (CQ-023), Q2-03 (MAT-003, R-049, tokens), Q2-04 (CQ-044) |
| Spec-adjacent (minor, fails closed) | 1: Q2-14 (CQ-024) |

Suggested order, by dependency. Q2-01 first, as a fix task under the acceptance criteria. Then
Q2-02 and Q2-03 (each small, each closes a spec gap). Q2-04 follows Q2-01, because Q2-01 adds
callbacks that belong in `Detail`. Q2-06 goes before Q2-07 and Q2-14. Q2-09 and Q2-08 go to the
security reviewer. Q2-05 goes after Q2-06 to Q2-08 so the moves do not clash. Q2-12 is one change,
so the imports change only once.

## 3. Phase 1 items: status at `551ee1da`

| Phase 1 ID | Status | Note |
| --- | --- | --- |
| CQ1-01, 14 | Open, grew | Now Q2-06 (seven copies). |
| CQ1-02 | Open | `events/page.rs:69-75` is unchanged: `from_timestamp_micros` accepts any instant chrono can hold. Still a behaviour fix and still small. Do it first. |
| CQ1-03 | Open | The three paging copies are unchanged (`devices/page.rs`, `events/page.rs`, `sessions/page.rs`). |
| CQ1-04 | Open, grew | Now Q2-07. |
| CQ1-05 | Partly done | `upkeep.rs` now exists as a wiring file and runs the events, metrics and access jobs. `run_daily` still lives in `events/upkeep.rs:57` (an `events` function that runs other domains' jobs), and the checks daily jobs are still in `checks/scheduler/daily.rs`. |
| CQ1-06 | Partly done | The leaf `targets.rs` exists. A second rule is still in `sessions/target.rs`. Now Q2-09. |
| CQ1-07, 11, 18 | Open, grew | Now Q2-12. |
| CQ1-08 | Open, grew | Now Q2-03. |
| CQ1-09 | Open, grew | Now Q2-04. |
| CQ1-10 | Open | Now Q2-08. |
| CQ1-12 | Open, grew | Now Q2-13. |
| CQ1-13 | Open | Now Q2-02. |
| CQ1-15 | Open, grew | Now Q2-14. |
| CQ1-16 | Open | `sessions/start.rs:56` (`type Display = guac_protocol::Display`) and `sessions/tickets.rs:153` still carry the Guacamole type into the SSH ticket. |
| CQ1-17 | Open, grew | 85 and 50 importers. In Q2-12. |
| CQ1-19 | Open | `lib.rs` is 243 lines and still holds the HTTP handlers (`is_json` at 188, `graphql` at 198, `subscriptions` at 233) and `serve_until` (165). |
| CQ1-20 | Open | Now Q2-15. |
| CQ1-21 | Open | `sessions/graphql/connection_test.rs` and `sessions/trust/connection_test.rs` are production files with a `_test` name. |
| CQ1-22 | Open | `guac-protocol/src/lib.rs:10` is unchanged. |
| CQ1-23 | Open | `AGENTS.md` still lists `pnpm -r`. The repository runs bun. |

## 4. Notes per lane

**Hub.** The domain shape from Section 13 holds. The pure cores are free of I/O: no `tracing`,
`sqlx` or clock read in `metrics/parse*`, `metrics/rates.rs`, `web_interfaces/rules.rs` or
`sessions/model.rs`. The `Instant::now()` calls are all inside relay task loops
(`sessions/terminal/*`, `sessions/tunnel/relaying/link.rs`, `sessions/files/relay.rs`), which are
the I/O edge. The 18 `let _ =` hits in the Phase 2 domains are all best-effort closes or sends on
channels whose receiver may be gone, each with a comment that says why. They are not listed,
though CQ-024 reads strictly. The debt is the same as in Phase 1, in the GraphQL layer: each new
domain (`web_interfaces`, `metrics`) copied the plumbing instead of sharing it (Q2-06, Q2-07). The
second debt is the width of `sessions/` (Q2-05).

**Client.** API access goes through one `api.ts` per domain and no component calls `fetch`
(CQ-045 holds). Every Phase 2 `catch {}` turns the failure into state or a message, or has a stated
reason (`open.ts:28`, `WebTab.svelte:251`, `transfer.ts:20`, `screen.ts:66,89,181`). The one soft
spot is `lib/sessions/files/link/upload.ts:61-65`, where a read error becomes an empty frame and a
later length check finds it. It works, but the error is not carried. The big finding is Q2-01: code
that was built and tested but never wired.

**Tauri shell.** `web_view/` is clean. Every command checks its caller, every error crosses as a
code only, and `unsafe` is confined to `macos_trust.rs` with a crate-level reason. The one item is
the fourth copy of process running (Q2-13). `web_view/commands.rs` repeats
`reported(caller(webview.label()).and_then(..))` seven times (lines 121 to 224). That is a small
duplicate and is not listed.

**`deploy/localapi/`.** `allow.ts` is pure and `server.ts` is the edge, as the file headers say.
Config is read once. The one item is the tuple cast (Q2-15).

## 5. What is already good

- `bun run quality:limits`, `quality:names` and `quality:imports` all print `ok`, and
  `quality/baseline.json` is `{}`. Phase 2 added 822 files without a baseline entry.
- Every Phase 2 hub domain has its own `thiserror` error, and no `anyhow` is used.
- The pure-core files (`metrics/parse`, `rates`, `reading`, `web_interfaces/rules`,
  `sessions/model`, `sessions/target`, `targets`) read no clock, database or environment.
- `targets.rs` is a good leaf: one address policy, tested, used by probes and sessions.
- The `credentials` vault binds every sealed value to its device and kind, and secret types have no
  `Debug` or `Serialize`. Only Q2-02 (the panic on a failed OS read) needs attention.
- The Tauri `web_view/` module: a second wall on the caller, codes without text, and a documented
  `unsafe` boundary.
- Client chart code is pure: `lib/charts/{plot,scales,ticks}.ts` take data and return data, and
  `lib/metrics/charts.ts` and `units.ts` do the same.
- `deploy/localapi/allow.ts` rebuilds every allowed call from validated parts, so the raw request
  never reaches `tailscaled`. It has one test file for the rule and one for the server.
- Every `#[expect]` and eslint disable in the Phase 2 code carries a reason.
