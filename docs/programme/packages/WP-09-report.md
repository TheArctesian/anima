# WP-09 report (package orchestrator)

**Status: done.**
- **Branch:** `claude/wp-09-checks`, with the latest `phase-1` (90218e0) merged in. The code tip is **f827121**, and this report is committed on top of it.
- **Pushed or merged:** neither.
- **Gates:** `BASE_REF=phase-1 bun run gates` passes **12/12** on f827121. Gates were also run after every task merge and after every phase-1 merge. Two batching tests had bounds that failed when the machine was busy and a DataLoader split a batch; both bounds were widened, and an unbatched read still fails them.
- **Plan:** the plan, decisions D-1 to D-9, the escalations and the full review log are in [WP-09.md](WP-09.md).

**What was not touched:** nothing was run against the VPS, Headscale or any cloud account, and nothing under `contracts/` was edited. The only shared files changed are the handoff lines listed below.

## What is delivered

| Task | Output | Spec |
| --- | --- | --- |
| T1 `checks-schema` | Migration 0300. Tables: `checks.checks` with its state columns (D-1); `heartbeats`, partitioned by UTC day with no default partition; `uptime_minute`, `uptime_hour` and `uptime_day`. Two `SECURITY DEFINER` partition functions with `search_path = pg_catalog, pg_temp`, executable only by `anima_runtime`. | DB-004, DB-005, DB-008, DB-009, DB-011, CHK-009 |
| T2 `maintenance` | `maintenance` domain. Migration 0400, plus 0401, which bounds the stored rules. Typed `WindowRule` (one-off, daily, weekly, monthly, every-N-days), with time zones and windows that cross midnight. A pure `is_active`. `active_for` and `active_targets`. GraphQL: the query and four audited mutations. | MW-001–004, API-003, LOG-005 |
| T3 `checks-core` | Typed configs; a pure `evaluate` (invert, retries, degraded threshold); status rules (pause, groups, maintenance, the manual flag); the store, where heartbeat writes upsert the aggregates in the same transaction; uptime from the coarsest table that covers the window; the face functions in K3. | CHK-002–006, CHK-008, CHK-011, DB-008 |
| T4 `probes` | ICMP, TCP, TLS (certificate days), DNS and HTTP probes. An address policy: refused ranges plus `ANIMA_PROBE_DENY`; every resolved address and every redirect hop is checked, and the probe connects to the address it checked. A 1 MiB body cap. Messages are cleaned and capped at 512 characters. No shell is ever started. | CHK-001, CHK-007, CHK-012 |
| T5 `scheduler` | `spawn_scheduler`, one task. Clock and RNG are parameters, and only the first run of each check gets jitter. Concurrency is bounded. `record_attempt` locks the row and evaluates the locked state. A status event is written before its Bus signal. The watchdog raises one Error event per lateness episode. Daily jobs: partitions ahead and behind, aggregate trimming, the orphan sweep. | CHK-001, CHK-002, CHK-010, HUB-003, HUB-005, DB-005, DB-009 |
| T6 `checks-api` | `checks::graphql`: query `check`; the `Check` fields; the heartbeats connection; create, update, delete and pause, all audited; the subscriptions `deviceStatusChanged` and `heartbeatAdded`. Sealed HTTP secrets bound to origin, header names, auth and TLS verification, and never returned. The push route `GET|POST /push/{token}`. DataLoaders for status, last heartbeat, uptime and heartbeat pages. | CHK-003, CHK-005, CHK-007, API-003, API-005, API-006, LOG-005, LOG-007, SEC-003, SEC-005 |
| T7 `device-fields` | The R-011 fields `Device.status`, `checks`, `uptime` and `ip` (R-026(3)), all through loaders. `DeviceFilter.status`. `DeviceSort.STATUS` and `UPTIME` per R-044, with cursors that are exact across ties. `createDevice(addDefaultCheck)` runs inside the device transaction. | CHK-004, CHK-011, API-005, API-006, R-011, R-026(3), R-044 |

**Review.** Every task was reviewed by a reviewer who did not build it. Each "accept" is backed by mutation: the reviewers and the orchestrator ran named mutations, and each one fails a named test. The final package review was an Opus security review of the whole diff. It found one must and six shoulds, all fixed in one round, and the re-review accepted the package. Details are in the review log in [WP-09.md](WP-09.md).

## Handoffs (shared files, PKG-001)

WP-09 does not edit `main.rs` or `schema.rs`. Until the lines below are wired, the scheduler, the push route and the checks and maintenance GraphQL roots cannot be reached in the running hub.

1. **`main.rs`: start the scheduler (R-030).**
   ```rust
   checks::spawn_scheduler(pool.clone(), bus.clone(), clock.clone(),
       checks::Probing { vault: vault.clone(), policy: AddressPolicy::new(config.probe_deny.clone()) });
   ```
   `AddressPolicy` is `checks::probe::AddressPolicy`. Only the deny list comes from config; the built-in refusals always apply.
2. **`config.rs`: `ANIMA_PROBE_DENY`.** A comma-separated list of CIDRs, parsed into `Vec<IpNet>`, empty by default. A bad entry must fail at start-up.
3. **`main.rs`: mount `checks::push_router(pool, bus, clock)`** behind the permitted-networks layer (NET-002). It needs a request timeout. Its URI must not be logged by any trace layer: log `/push/[redacted]` only (LOG-007, R-028(2)).
4. **`schema.rs`: merge the roots.**
   - `checks::graphql::{Query, Mutation, Subscription}`
   - `maintenance::graphql::{Query, Mutation}`
5. **`schema.rs` `.data()`: register the loaders.**
   - `devices::graphql::status_loader(pool, clock)`
   - `devices::graphql::uptime_loader(pool, clock)`
   - `devices::graphql::checks_loader(pool)`
   - `devices::graphql::resolved_ip_loader(pool)`
   - `checks::graphql::check_status_loader(pool, clock)`
   - `checks::graphql::last_heartbeat_loader(pool)`
   - `checks::graphql::heartbeats_loader(pool)`
   - `checks::graphql::check_uptime_loader(pool, clock)`
6. **Root and hub `Cargo.toml`** (already in the branch; review at merge, E-6):
   - `chrono` gains the `serde` feature, and tokio gains `io-util`.
   - New dependencies: `chrono-tz`, `surge-ping`, `hickory-resolver`, `reqwest` (rustls), `rustls`, `tokio-rustls`, `x509-parser`, and the dev-dependency `rcgen`.
   - **`rand` moves to 0.10 for the whole workspace.** WP-10's test server and the WP-09 scheduler had pulled in two versions that clash (E0464). The hub's separate dev-dependency line `rand = "0.10"` is removed, and the scheduler imports `RngExt`.
7. **`events/model.rs`:** `HeartbeatSummary.message: Option<String>`, and the type is now Clone rather than Copy (R-036).
8. **WP-05 / main.rs:** the events partition jobs still need to be run from `main.rs`, as listed in K3.

## Deferred (for issue #1)

- A shared text-rule leaf, so the checks, maintenance and devices name and text rules are not written three times.
- The in-memory device listing for STATUS and UPTIME sorts loads every matching device. It is fine at Phase 1 scale, but it should become an SQL join, and cursors are positional (documented in `listing.rs`).
- `push_token` is stored in plaintext. Seal it, plus keep an HMAC for lookup.
- Custom secret header names beyond Authorization and Cookie.
- `events::record` publishes before its transaction commits (seen from WP-09, owned by WP-05).
- **Package review C-a (CHK-001):** one unreadable window row stops all recording, because scheduled attempts back off and pushes get 500. Ideas: make 0401 require whole numbers, add a time-zone CHECK, or skip a bad row and log an error.
- **Package review C-b:** a device deleted mid-push gives 500 where T6 chose 404. Map `Error::Devices(NotFound)` to 404.
- **Package review C-c:** a check deleted between tick and attempt logs at warn in `observe_http` but at debug in the record step.
- `regroup` writes one `update_check` per child. It is capped at 100 children.
- No deterministic test proves the row locks (`FOR UPDATE`) in the checks write paths. The T5 scheduler locks are proved by concurrency tests.

## Open questions

1. With the WP-09 fix, the 100.100.100.200 gap is closed: all of `100.100.100.0/24`, which includes Tailscale's quad-100, is refused by default, together with `192.0.0.0/24` and `168.63.129.16`. If the operator ever wants to probe a quad-100 service, that needs an allow list. Today there is only a deny list.
2. D-5 now applies maintenance to pushed beats as well. A `down` pushed during a window is stored as Maintenance and does not count against uptime, which matches scheduler beats. Please confirm that this is the intended reading of CHK-006 for PUSH checks.
3. Some phase-1 code comments cite ruling IDs (for example R-019). WP-09 removed all plan and ruling IDs from its own Rust comments under COM-002, but left the migrations alone to keep their checksums. Please confirm whether ruling IDs are allowed in comments or not.
