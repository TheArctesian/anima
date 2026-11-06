# Phase 1 code-quality audit (CQ-050)

**No item in this report has been applied.** This is an audit-mode report: no source code was
changed. Each item needs the operator's acceptance first. An accepted item is then built on its own
branch and pull request (CQ-050), moving code without a rewrite (CQ-051), with the gates run and the
baseline lowered where it applies.

Audit of `phase-1` code at `228f5d1` (branch `wp/17-acceptance/quality-audit`), WP-17 T5. The
auditor did not build this code.

## 1. Method and scope

| Item | Value |
| --- | --- |
| Skill | `code-quality` (CUPID), audit mode. The skill's multi-agent workflow was not available in this session, so its lanes were run one after another by one auditor. There was no separate verifier vote. Each item below was checked against the code at the cited lines, and the auditor re-read the cited code before writing it up. |
| Rules | `AGENTS.md`; design spec Section 12 (CQ-001 to CQ-051) and Section 13 (REPO-001, REPO-002, the domain folder shape). |
| Gates already passing | `bun run quality:limits`, `quality:names` and `quality:imports` all print `ok` at this commit. `quality/baseline.json` is `{}`. So the audit does not repeat size, banned-name or import-boundary findings. It looks for what those gates cannot see: units with more than one purpose, names that do not tell the truth, I/O inside pure cores, untyped or swallowed errors, single-use abstractions, logic copied across domains, folders that are not operator nouns, and exports nobody uses. |
| Signals swept | Repository greps for `let _ =`, `.ok()`, `unwrap_or_default`, `catch {`, `as unknown as`, clock, random and environment reads (`Utc::now`, `Instant::now`, `rand`, `getrandom`, `env::var`), K7 error-code literals, and duplicate function names across domains (`fn internal`, `fn pool`, `fn begin`, `Page<T>`, `encode_cursor`, `struct Trail`). Every pure-core file (`rules.rs`, `model.rs`, `schedule.rs`, `status.rs`, `evaluation.rs`, `uptime.rs`, `page.rs`, `text.rs`, `targets.rs`, `active.rs`, `checks/config/*`, `events/redaction/*`) was checked for `PgPool`, `sqlx`, `tracing` and `tokio`: none use them. |
| Lanes | Hub (`crates/hub/src`, by domain), `crates/guac-protocol`, `crates/theme`, client (`apps/client/src/lib`, `routes`), Tauri shell (`apps/desktop/src-tauri`), `deploy/` and `quality/` TypeScript. |
| Excluded | Phase 2 work, such as the unmounted Guacamole `/tunnel` path, which is kept only for RDP and VNC (its own findings are left out; it appears only where it duplicates Phase 1 code). Items already on GitHub issue #1, from the "For issue #1" and "Deferred" sections of `docs/programme/packages/WP-*-report.md`. Those include the four device id parsers (WP-06 C3), the loader types in two places (WP-06 C4), the shared text-rule leaf (WP-09), `events::record` publishing before commit (WP-09), the COM-002 comment sweep (WP-07), `lib/hub/socket/fake.ts` (WP-11), `address.ts` placement and `AddressResult` (WP-16), the dynamic `import()` lint gap (WP-11, WP-18), the host-key `<dialog>` (WP-14), and the R-030 session deny list (WP-10). The motion and narrow-width findings in `docs/research/motion/phase-1.md` are being fixed separately, so they are also left out. |
| Ranking | By value to the operator, then by risk: must (behaviour or observability at stake), should (duplication or misplaced logic that will drift), could (polish). Sizes: S is under half a day and one or two files; M is one to two days or several files; L is more than that. |

## 2. Ranked items

| ID | Property | Location (evidence) | Proposal | Risk | Size |
| --- | --- | --- | --- | --- | --- |
| CQ1-01 | P | Five copies of the K7 error vocabulary: `crates/hub/src/checks/graphql/error.rs:10,41-75`, `devices/graphql/error.rs:9,26-60`, `maintenance/graphql/error.rs:7,24-50`, `sessions/graphql/error.rs:7,26-60`, `events/graphql.rs:28,190-206`. Each has the same `INTERNAL_MESSAGE`, `internal`, `coded`, `invalid`, `schema_dependency` and `UnregisteredDependency`. The K7 codes are bare string literals in 10+ places: `access.rs:112-121`, `checks/graphql/input.rs:97`, `devices/graphql/credential_input.rs:94`, `sessions/graphql/connection_test.rs:76`. The process log matches on those strings to classify errors: `events/process_log/internal_errors.rs:22-23`, `echoes.rs:20`. | **Move** the shared constructors into one new leaf, for example `crates/hub/src/api_errors.rs`, with a typed `ErrorCode` enum for the K7 codes. Add it to `quality/hub-modules.json` `leaves`. Each domain keeps its own `to_graphql(Error)` mapping (API-010). `process_log` then matches on the enum, not on text. | Low to medium. The wire codes must not change; the existing K7 tests pin them. | M |
| CQ1-02 | P | `crates/hub/src/events/page.rs:69-75`: `decode_cursor` accepts any instant chrono can hold. `sessions/page.rs:18-20,92-94` bounds the instant to what PostgreSQL `timestamptz` binds (API-010), and `checks/graphql/heartbeat.rs:17-18,57-61` bounds it by year. An events or audit cursor whose time is before 4713 BC likely reaches the database and comes back `INTERNAL`, not the `after` `VALIDATION` error. This was found by reading the code; no test has run it. | **Fix** in a separate change from CQ1-03, first. Bound the micros the way `sessions/page.rs` does. Add an `api_006` test with a cursor below the range. | Low. | S |
| CQ1-03 | C | The same paging code three times: `Page<T>`, `page_size`, `split_page` and `MAX_PAGE_SIZE` in `crates/hub/src/devices/page.rs:7-75`, `events/page.rs:8-45` and `sessions/page.rs:15-109`. The (instant, id) newest-first keyset cursor is written two ways: events uses `e1.<micros>.<hex>` at `events/page.rs:57-75`, sessions uses `s1.<base64>` at `sessions/page.rs:60-99`. Heartbeats use a third codec at `checks/graphql/heartbeat.rs:53-75`. `paging.rs:1` says it is "shared by every list" but holds only `PageEnd`. | **Move** `Page<T>`, `fetch_limit` and `split_page` into `paging.rs`, plus `page_size(first, max)`, which returns the field error. Put one bounded (instant, id) cursor codec there too, with a prefix for each list. Devices keep their name cursor. | Medium. Changing a cursor format makes old cursors invalid. They are opaque and short-lived, but say so in the pull request. | M |
| CQ1-04 | U | The same `Trail` (`from_context`, then audit write, then event write) in three domains: `crates/hub/src/checks/graphql/trail.rs:28-85`, `devices/graphql/trail.rs:27-85` and `maintenance/graphql/trail.rs:24-80`. Only `Source` and `device` differ. | **Move** `Trail` and its `write` into the events domain face, for example `events::Trail::write(conn, entry)`, with the source on the entry. Each domain keeps its pure `Entry` builders (`created`, `window_added` and so on). | Low. One transaction and the same order (audit, then event). | M |
| CQ1-05 | U | The check scheduler also runs the heartbeat retention jobs (`crates/hub/src/checks/scheduler/tick.rs:93-96` calls `daily::run`, `scheduler/daily.rs:17-40`), with its own daily timing (`scheduler/schedule.rs:94-104`). A second daily-job runner, `events::run_daily` (`events/upkeep.rs:21-57`), runs the event partitions and *access* pruning from `main.rs:253-280`. So the hub has two daily-job policies (00:10 UTC with doubling retries, against a day after success and an hour after failure), and an `events` function runs another domain's job. Spec 12.1: "The check scheduler schedules. It does not probe, store or notify". | **Move** `upkeep.rs` out of `events` into a leaf, then **move** the checks daily jobs into `main.rs::spawn_upkeep` as one more `run_daily` job. The scheduler keeps only the check ticks. | Medium. Heartbeat partitions must still exist before the first insert. Today the first tick runs them, so run the job once at start, as `run_daily` already does. | M |
| CQ1-06 | P | Two separate rules for "may the hub connect to this address": `crates/hub/src/checks/probe/reach.rs:18-100` (`AddressPolicy`, `embedded_v4`, `canonical`) and `sessions/target.rs:7-65` (`is_refused_target`, `embedded_v4`, `six_to_four_v4`). Each decodes IPv4-in-IPv6 its own way and refuses a different set. | **Merge** into one pure address rule that both domains call, with each caller's extra refusals as data. Do it together with the WP-10 issue #1 item on the R-030 deny list, which changes the same rule's contents. | High (SSRF boundary; security review). | M |
| CQ1-07 | D | `apps/client/src/lib/forms/` holds domain forms, not form primitives (spec 13 says forms/ is "Form primitives and validation"): `forms/check/` (the whole check editor model and fields), `forms/device/` (the device form, host rules, connection test), `forms/AddDeviceModal.svelte`, `forms/MachineChoice.svelte` and `forms/this-machine.ts`. | **Move** `forms/check/` to `$lib/checks/form/`, and `forms/device/`, `AddDeviceModal`, `MachineChoice` and `this-machine.ts` to `$lib/devices/form/` (or `devices/add/`). `forms/` keeps `FormButton`, `modal/` and `disclosure/`. Fix the imports in about a dozen files (routes and `lib/sessions/view/pane/LinkConfirm.svelte`). | Low (moves only). Watch the 12-files-per-folder limit in `lib/devices/`. | M |
| CQ1-08 | I | Two focus traps with different rules: `apps/client/src/lib/forms/modal/focus.ts:1-37` (its selector includes `select` and `textarea` and filters out `hidden` and `aria-hidden`) and `lib/sessions/view/hostkey/focus.ts:1-47` (its selector omits `select` and `textarea` and does not filter hidden elements). There are also two `HostKeyModal.svelte` components: `forms/modal/HostKeyModal.svelte` is built on `Modal.svelte`, and `sessions/view/hostkey/HostKeyModal.svelte` brings its own trap. | **Merge** into one focus trap in `forms/modal/focus.ts`, and have the session warning use it, or `Modal.svelte`. The two modals stay separate, because they do different jobs: one accepts a key after a test, the other warns of a changed key mid-session. Do it with the WP-14 `<dialog>` item if that is accepted. | Medium (keyboard behaviour; interface review). | M |
| CQ1-09 | U | The device detail route holds the mutation and state-patching logic: `apps/client/src/routes/devices/[id]/+page.svelte:84-127` (`saveSettings`, `acceptHostKey`, `clearCredential`, `deleteDevice`, `putCheck`, `dropCheck`), against CQ-044. It also has `id ?? ''` at lines 45, 98, 104 and 113, which would send an empty id to the hub where the page already knows the id is `null`. | **Move** these into `Detail` (`detail.svelte.ts`) as methods that take the domain APIs. Build `Detail` and the session `Pages` only for a non-null id, so the `?? ''` fallbacks go away. | Low to medium (route tests cover the flows). | M |
| CQ1-10 | C | The terminal and Guacamole session tasks are line-for-line copies: `run` and `session` in `crates/hub/src/sessions/terminal/session.rs:37-72` and `sessions/tunnel/relaying.rs:46-81`. The race-the-end send is also copied: `terminal/writes.rs:25-38` and `tunnel/relaying/writes.rs:25-40`. | **Move** the shared skeleton (redeem, check the subprotocol, `handed_over`, `finish`) and the race-send into `sessions/upgrade.rs`, which already holds `handed_over` and `upgraded`. Each tunnel passes its `open` and `connected`. | High (session tunnel; security review). The Guacamole path is unmounted, so only its tests exercise it. | M |
| CQ1-11 | D | Each desktop bridge sits apart from its domain: `apps/client/src/lib/desktop/processLog.ts` against `lib/process-log/LogFolderRow.svelte`; `desktop/updater.ts` against `lib/updates/`; `desktop/thisMachine.ts` against `lib/forms/this-machine.ts`; `desktop/systemSettings.ts` and `desktop/RemoteLoginButton.svelte`. The "this machine" concept is in two folders, with two spellings. | **Move** each bridge beside its domain (`process-log/`, `updates/`, a `this-machine/` folder), keeping the Tauri `invoke` calls at that edge. `address.ts` stays out of this item (WP-16 issue #1). | Low (moves). | M |
| CQ1-12 | P | The Tauri shell runs an OS program three ways: `apps/desktop/src-tauri/src/process_log/folder.rs:28-58` (polls, then reaps after a wait), `this_machine/name.rs:47-80` (polls, then kills) and `system_settings.rs:50-65` (`.status()`, which blocks with **no** wait limit, though `folder.rs:18-20` explains why openers need one). | **Merge** into one shell module, for example `os_program.rs`, that runs a program with a wait limit. `system_settings` then gets one too. | Low. | S |
| CQ1-13 | I | Two ways to get secret random bytes: `crates/hub/src/checks/graphql/write.rs:41-46` uses `rand::fill`, which panics if the OS generator fails, while `access/login.rs:132` and `sessions/start.rs:124` use `getrandom::fill` with a typed `Error::Random` (`access.rs:78`, `sessions.rs:103`). The push token is also made in the GraphQL file, not in the checks domain. | **Move** `fresh_token` into the checks domain, using `getrandom` and a `checks::Error::Random`. | Low. | S |
| CQ1-14 | I | `pool`, `begin` and `commit` are copied: `crates/hub/src/devices/graphql.rs:102-119`, `maintenance/graphql.rs:23-40` and `checks/graphql/write.rs:22-38`. A second `pool` sits in the same domain at `checks/graphql/check.rs:28`. | **Merge** into the CQ1-01 leaf, generic over the domain's `to_graphql`, or at least drop the second checks `pool`. Depends on CQ1-01. | Low. | S |
| CQ1-15 | P | `crates/hub/src/checks/store/sealed.rs:43,78`: `origin(..).unwrap_or_default()` and `serde_json::to_vec(&Binding).unwrap_or_default()` seal an empty binding without saying so. It fails closed later (`permits`, line 59), but it surfaces as `SecretsUnbound` at probe time, not at the save that caused it. | **Change** `to_vec` to an `expect` that states the invariant (plain strings and bools always serialise, CQ-023), and have `seal` return a `Result` when the URL has no origin. | Low. | S |
| CQ1-16 | D | Session tickets carry `guac_protocol::Display` (`crates/hub/src/sessions/tickets.rs:118`, `sessions/start.rs:48`), and the SSH start checks time zones with `guac_protocol::is_zone_name` (`start.rs:59-66`). SSH no longer goes through guacd (R-062), so a Guacamole crate type is the session domain's display type. | **Move**: sessions owns a `Display` (size, DPI, zone) in `sessions/model.rs`. The Guacamole tunnel converts it to `guac_protocol::Display` in `tunnel/arguments.rs`. The crate keeps `is_zone_name` for its own handshake refusal. | Low. | M |
| CQ1-17 | I | `apps/client/src/lib/hub/hub.ts:8-15` re-exports `error`, `address`, `request` and `status` types. Callers import `HubResult` and `HubError` both ways: 54 imports `from '$lib/hub/hub'` and 35 `from '$lib/hub/error'`. | **Choose** one face (`hub.ts`) and add an import restriction so internals are not imported straight. Or drop the barrel. One change everywhere. | Low. | S |
| CQ1-18 | I | Client file names mix spellings: camelCase in `lib/desktop/thisMachine.ts`, `processLog.ts` and `systemSettings.ts`; kebab-case everywhere else (`forms/this-machine.ts`, `forms/add-device.ts`, `access/sign-out.ts`, `forms/check/model/config-input.ts`, `validate-http.ts`, `hub-types.ts`, `forms/device/field-errors.ts`, `connection-test.ts`). | **Rename** the three camelCase files to kebab-case, together with CQ1-11, and add the casing rule to `quality:names`. | Low. | S |
| CQ1-19 | U | `crates/hub/src/lib.rs:41-200` is the module tree, and also the router, CORS, the GraphQL POST and WebSocket handlers (`is_json`, `graphql`, `subscriptions`) and the drain (`serve_until`). | **Move** the route handlers and `serve_until` into a wiring module named for the edge, for example `http.rs`, listed under `wiring`. `lib.rs` keeps the tree. | Low. | S |
| CQ1-20 | P | Unchecked casts at a parse boundary: `quality/src/hub-modules.ts:46` (`parsed as Record<string, unknown> \| null`, while `quality/src/baseline.ts:41` uses an `isRecord` guard for the same job) and `deploy/dns/address.ts:32` (`octets as unknown as Ipv4`). | **Change** both to use a guard: reuse `isRecord`, and build the tuple from four checked numbers. | Low. | S |
| CQ1-21 | D | `crates/hub/src/sessions/graphql/connection_test.rs` and `sessions/trust/connection_test.rs` are production modules (the `testConnection` mutation and its probe), but a `_test` suffix reads as a test module in Rust. | **Rename** them after the K1 mutation, for example `test_connection.rs`. | Low. | S |
| CQ1-22 | C | `crates/guac-protocol/src/lib.rs:10` exports `MAX_ELEMENTS` and `MAX_LENGTH_DIGITS`. Only the crate's own `tests/it` use them. `MAX_INSTRUCTION_BYTES` has a hub user, `sessions/tunnel/guacd.rs`. | **Keep** the exports, but say in their docs that they are the test contract, or reach them through a `#[doc(hidden)]` test face. | Low. | S |
| CQ1-23 | I | `AGENTS.md` "Gates" lists `pnpm -r lint && pnpm -r check && pnpm -r test`, and spec Section 13 says `package.json` is a pnpm workspace. The repository runs bun: `package.json` scripts, `bun run gates`, `quality/src/gates/gates.ts:25-41`. | **Change** `AGENTS.md` to `bun run gates`. The spec Section 13 wording needs the operator's ruling, because the spec is the source of truth. | Low. | S |

### Counts

| Property | Items |
| --- | --- |
| C (Composable) | 3: CQ1-03, 10, 22 |
| U (Unix philosophy) | 4: CQ1-04, 05, 09, 19 |
| P (Predictable) | 6: CQ1-01, 02, 06, 12, 15, 20 |
| I (Idiomatic) | 6: CQ1-08, 13, 14, 17, 18, 23 |
| D (Domain-based) | 4: CQ1-07, 11, 16, 21 |

| Lane | Items |
| --- | --- |
| Hub | 13: CQ1-01 to 06, 10, 13, 14, 15, 16, 19, 21 |
| `guac-protocol` | 1: CQ1-22 (CQ1-16 also touches it) |
| `theme` | 0 |
| Client | 6: CQ1-07, 08, 09, 11, 17, 18 |
| Tauri shell | 1: CQ1-12 |
| `deploy/` and `quality/` TypeScript | 1: CQ1-20 |
| Repository rules | 1: CQ1-23 |

Suggested order, by dependency: CQ1-02 first (a behaviour fix, small), then CQ1-01 before CQ1-14,
CQ1-03, CQ1-04 and CQ1-05. CQ1-06 and CQ1-10 go to the security reviewer. The client moves CQ1-07,
CQ1-11 and CQ1-18 go together, so imports change only once.

## 3. Notes per lane

**Hub.** The domain shape from Section 13 holds. Every pure-core file is free of I/O. Time comes
through `Clock` (`clock.rs`), and randomness is passed into the scheduler. Every domain has its own
`thiserror` error, and no `anyhow` is used. Most of the debt is in the GraphQL layer, where each
domain grew its own copy of the same API plumbing: errors (CQ1-01), transactions (CQ1-14), paging
(CQ1-02, CQ1-03) and the audit trail (CQ1-04). Moving these into leaves will stop them drifting;
CQ1-02 is a case where they already have. The `let _ =` hits are all documented best-effort closes
or sends on channels whose receiver may be gone. They are not swallowed errors and are not listed.
`access/listen/` was considered for its own domain, but it uses the access layer and
permitted-networks rules, so the boundary is defensible. `sessions/` is the widest domain (11 flat
files beside six sub-modules). It is within limits, and no split is proposed until Phase 2 adds
RDP.

**`guac-protocol`.** It is a clean I/O-free codec, as CQ-020 asks. The only point is the test-only
exports (CQ1-22). Its `Display` leaking into the session domain is listed under the hub (CQ1-16).

**`theme`.** No items. It is a small generated palette and one formatter. Its only hub user,
`guac_color_scheme`, is on the unmounted Guacamole tunnel (Phase 2).

**Client.** API access goes through one `api.ts` per domain, and no component calls `fetch`
(CQ-045 holds). Every `catch {}` that was found turns the failure into state or a message, or is a
storage refusal with a stated reason, so none is listed as swallowed. The findings are about
placement: domain forms under `forms/` (CQ1-07), desktop bridges apart from their domains (CQ1-11),
and logic in the detail route (CQ1-09). There are also two local-idiom splits: the focus traps
(CQ1-08) and the hub import paths (CQ1-17). The Guacamole client files (`sessions/tunnel/guacamole/`,
`sessions/view/guacamole/`) are kept for Phase 2 and were not audited.

**Tauri shell.** It holds only OS work, as REPO-003 asks. Errors cross to the web view as a code and
fixed text. The one item is the three copies of process running, one of them with no time limit
(CQ1-12).

**`deploy/` and `quality/` TypeScript.** These are small, pure parsers, each with its own test. The
two duration parsers in `deploy/headscale/src/` look alike but are two real grammars (Go and
Prometheus), so they are not listed. There is one cast item (CQ1-20). The gate runner takes its
process runner and clock as parameters (`quality/src/gates/gates.ts:47-52`).
