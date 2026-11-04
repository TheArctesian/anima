# WP-10 report (package orchestrator)

Status: **done**. Branch `claude/wp-10-sessions`, with the latest `phase-1` (`b443d15`, R-040) merged in. Not pushed, not merged.
`BASE_REF=phase-1 bun run gates` on `d21b239` (code tip; the commit that adds this file changes only docs): **12/12 pass**, exit 0, `DATABASE_URL` unset. Plan, decisions D-1..D-13, rulings R-032..R-040 and the review log: [WP-10.md](WP-10.md).

## What is delivered

`crates/hub/src/sessions/` and migrations 0600–0601: the sessions domain (spec Sections on remote access; K1, K3 `## sessions`, K4 `contracts/tunnel.md`).

| Task | Output | Spec |
|---|---|---|
| T1 records | `sessions.sessions` and `sessions.host_keys` (0600), `Query.sessions` with keyset pages, `host_key`, `host_keys` (one query), `close_stale` | RA-008, SEC-006, API-005 |
| T2 tickets | `Live` (tickets, tunnels, observations), `start` / `startSession` (32-byte single-use ticket, SHA-256 kept, 30 s, bound to operator, session and client address), `endSession`, sweep, target refusal | K4 §1, API-009, SEC-002, LOG-005, R-032 |
| T3 host keys | russh probe (`read_host_key`), `testConnection`, `acceptHostKey`, first-seen observations, `TargetRule` (single home of the target rule), `SshSettings.hostKey` with a batch loader | DEV-013, SEC-006, SEC-007, R-011, R-034 |
| T4 relay rules | connect values, `known_hosts` line, client allow-list, gateway rules (`error` with fixed text), end table | GW-011..015, R-035 |
| T5 tunnel | `GET /tunnel` (`guacamole` subprotocol), the opening in its own task with an `Unfinished` guard, hub-side host-key probe before guacd, guacd handshake through guac-protocol, relay with size bounds, ping, `nop`, idle wait, close codes and `endReason`, end records (one end, audit and event) | K4 §1–§7, GW-010..015, SEC-002, SEC-007, MV-003, LOG-007, R-034, R-039 |
| Final | audited stale and shutdown ends, `sessions::shut_down`, end-once trigger, finite and ordered times, column grants, time zone bound with UTC fallback | LOG-005, RA-008, D-10, D-13, R-040 |

## Review summary

Every task: builder-heavy (Opus); reviewer-security (Opus) and reviewer (Sonnet) with mutation testing in non-git scratch copies. Final package review (Opus, security) of `phase-1...claude/wp-10-sessions`, then a final round re-reviewed by security. All accepted. Musts found and closed: T2 `endSession` double lock (redeem race); T5 client leaving during the opening left the row live; T5 `endSession` during the opening still sent the credential. Details per round in the review log.

## Handoffs (PKG-001)

H-2 wiring in `main.rs` / `lib.rs` / `schema.rs` (this package edits none of them). **The `/tunnel` mount is left out of this merge and waits for WP-05 T4** (`access::permitted_networks_layer` is not on phase-1; R-040). Each item is testable:

1. Exactly one `sessions::Live::default()`, cloned into the schema (`.data(live.clone())`) and into `tunnel_router`. Missed: session mutations return INTERNAL; two `Live`s make every tunnel 403.
2. Merge `sessions::graphql::{Query, Mutation}` and add `.data(sessions::graphql::host_key_loader(pool))`. Missed: `ssh { hostKey }` returns INTERNAL.
3. After WP-05 T4: `sessions::tunnel_router(pool, vault, bus, clock, config.guacd, live)` merged at `/tunnel` with `access::permitted_networks_layer` applied. Test: a valid token from a non-permitted address is refused and logged, and still redeems from the permitted address. Missed: NET-002 broken.
4. No URI or query in that layer's refusal event or logs, and no `TraceLayer` (or anything recording `uri`) over `/tunnel`: the token is in the query (LOG-007).
5. Serve with `ConnectInfo<SocketAddr>` (kept when TLS serving lands); `ClientAddr` the same for `/graphql` and `/tunnel`, never from a header.
6. `sessions::close_stale(&pool, clock.now())` once, after the runtime role check and before serving; one hub per database.
7. On the shutdown signal, before the runtime stops: `sessions::shut_down(&pool, &live, clock.now())`, then the server's graceful drain (upgraded tunnels are not ended by the drain). Log an `Err`.
8. H-3: `config.guacd` (`ANIMA_GUACD`, default `guacd:4822`) exists; GW-002 (4822 not published) is WP-04 compose.

## Contract change requests (CON-001) / open questions

All raised and ruled: E-1..E-8 (R-032), R-033, E-9/E-10 (R-034), R-035, E-12 (R-038), E-13 (R-039), E-14 (R-040). None open.

## Deferred (for issue #1, R-007)

- R-030 deny list for sessions: the target rule still allows the hub's Docker networks (bridge gateway 172.17.0.1:22, the VPS public address, other containers).
- `ArgValues` copies of secrets are not zeroed (live in the handshake up to about 20 s).
- russh's password `String`, and the GraphQL body/JSON of a `testConnection` credential, are not zeroized (cross-cutting with WP-06).
- H-10: compare a real guacd `args` with the fixtures; refuse an `args` list without `host-key`.
- A multi-key-type test against a real guacd (its libssh2 backend decides the key types).
- Blocking DNS lookups; no concurrency bound on `testConnection` probes or on open tunnels.
- Port oracle through `testConnection` outcomes (operator-only).
- A client that sends a byte after the request headers and then leaves is seen only at the handshake wait (hyper EOF detection); documented and pinned by a test.
- Head-of-line stall in the single-task relay between the two writes (every write still races the end signal).
- F3: the tunnel uses the first checked address while the probe tries several (fails closed).
- F4: an unsaved `testConnection` against an endpoint a saved device pins shows no "changed" warning.
- F5: the `finish` retry can publish an event twice (WP-05 publishes before commit).
- An `endSession` landing between a start's commit and its ticket insert can leave the row live until `close_stale`.

## Notes

- Edits outside `sessions/`: WP-06 devices paths for `SshSettings.hostKey` and the input helpers (R-038); `russh`, `rand` (dev) and `tungstenite` manifest lines (R-038, R-039). The lockfile adds only russh's crates.
- One security reviewer split a hostname to fetch a guacd source file past a guard during T4; the classifier then refused further fetches. Not repeated; later reviewers were told not to work around guards.
