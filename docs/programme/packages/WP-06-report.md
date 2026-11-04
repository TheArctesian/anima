# WP-06 report (package orchestrator)

Status: **done**. Tasks T1 to T6 are accepted, the final package review (Opus, security) is accepted after a fix round, and `bun run gates` passes 12/12 on the branch tip. Not pushed, not merged into `phase-1` or `main`.

| Item | Value |
| --- | --- |
| Branch | `claude/wp-06-devices-b9e5ba` |
| Tip | `TIP` (includes `phase-1` at `c2a2b0c`, WP-05's log-capture fix) |
| Already on `phase-1` | T1 migrations and fixtures (`525078f`, merged as `668c898`) |
| Plan and review log | [WP-06.md](WP-06.md) |
| Spec | DEV-010–015, VPN-001, VPN-002, API-003, API-005, API-006, API-010, SEC-003, SEC-005, LOG-004, LOG-005, LOG-007 |
| Contracts | K1 (devices part; not `testConnection`/`acceptHostKey` and not the R-011 fields), K2 `devices`, K3 `devices`, K6 fixtures, K7 error codes |

## Gates (`env -u DATABASE_URL bun run gates`, tip `TIP`)

| # | Gate | Result |
| --- | --- | --- |
| 1 | cargo fmt --check | pass |
| 2 | cargo clippy --all-targets -- -D warnings | pass |
| 3 | cargo test (throwaway PostgreSQL, R-009, R-019) | pass |
| 4 | bun install --frozen-lockfile | pass |
| 5 | bun run lint | pass |
| 6 | bun run check | pass |
| 7 | bun run test | pass |
| 8 | client build | pass |
| 9 | quality:limits | pass |
| 10 | quality:names | pass |
| 11 | quality:imports | pass |
| 12 | quality:baseline --base phase-1 | pass |

Before WP-05's fix, gate 3 failed on every run of 7ddadcd, and only on WP-05's process-log test. The amended ruling covered that, and the plan records it under "Gate reruns". Every run since the fix was merged (b5479cd and later) passed 12/12 with no reruns.

## What the package delivers

- **Schema** `devices` (migration `0100`): `sites` with an exclusion constraint against overlapping subnets (VPN-001), `devices`, `device_tags` and `ssh_settings` cascading from the device (DEV-015), named constraints mapped to field errors.
- **Face** `devices.rs` (K3): `get`, `list`, `in_maintenance`, `Device`, `Site`, `SshSettings`, a typed `Error` whose Display names fields, never values.
- **Pure core**: `rules.rs` (DEV-010 host, SSH settings, tags, input caps D-12), `text.rs` (host labels; refuses control, bidi, line-separator and direction-mark characters), `page.rs` (keyset cursors by name in byte order, `first` 1–200, API-006).
- **Store** `store.rs`: the only SQL; writes in savepoints; keyset list with name, tag and site filters and `totalCount`.
- **GraphQL** (`devices/graphql.rs` and `devices/graphql/`):
  - queries `sites`, `device`, `devices` and `tags`;
  - mutations `createSite`, `createDevice`, `updateDevice` and `deleteDevice`;
  - `setSshCredential` and `clearSshCredential`;
  - `SshSettings.credentialStored`.
  
  Every root field has `Authenticated` (R-022). Loaders cover `Device.site`, `Site.devices` and `credentialStored` (API-005, R-017). Each mutation writes the audit first and then the event, inside its transaction (LOG-005).
- **Credentials** (R-010): the secret becomes `Secret` when the argument is parsed. A hand-written `SshCredential` input type never quotes a credential that does not parse. That closes a leak found here: async-graphql's derived parse errors include the whole value, and the process log writes them. `store_ssh`/`clear_ssh` run before any lock on the device row, and `deleteDevice` clears the secret first, which fixed a proven deadlock against `setSshCredential`.
- **Tests**: lib unit tests and `tests/it/devices/` integration tests on a throwaway database with the K6 fixtures. They cover the K1 surface both ways, the guard over all 10 root fields, constant statement counts for 13 vs 163 devices, rollback on a failed audit, and no secret in any row, response, bus message or log. They also cover the concurrent set vs delete race. Each review round's mutations fail a named test (review log).

## Handoffs (PKG-001)

| ID | To | Item |
| --- | --- | --- |
| H-1 | WP-01 (accepted) | Cargo features: sqlx `runtime-tokio`, `migrate`, `macros`, `chrono`, `ipnet`; async-graphql `chrono`, `dataloader`. |
| H-2 | Integration (`schema.rs`) | Wiring lines below. |
| H-4 | WP-09, WP-10 | The `Device`/`DeviceSsh` seams, credential input rules and lock order; see WP-06.md H-4. |
| H-5 | WP-05 | Log the hidden source of every INTERNAL error at the API boundary. Today an INTERNAL with a code is neither logged nor masked, so a deadlock or vault failure leaves no log line. |
| H-6 | WP-05 | (a) Refuse unauthenticated requests before argument parsing. (b) A body limit. (c) Redact event messages. (d) Stop the redaction of every `auth` key. (e) Replace parser syntax-error messages, which echo the query line and so any inline credential, with a fixed text, and have the client send credentials only as variables. (f) Unknown input field names are echoed to the sender. |

### `schema.rs` wiring (H-2)

```rust
// roots
Query(.., devices::graphql::Query, ..)
Mutation(.., devices::graphql::Mutation, ..)
// schema data
.data(pool.clone())                                          // PgPool
.data(vault)                                                 // credentials::Vault from ANIMA_KEY_FILE (SEC-004)
.data(bus.clone())                                           // events bus
.data(clock)                                                 // Arc<dyn Clock>
.data(devices::graphql::site_loader(pool.clone()))           // DataLoader<SitesById>
.data(devices::graphql::devices_by_site_loader(pool.clone())) // DataLoader<DevicesBySite>
.data(devices::graphql::credential_loader(pool.clone()))     // DataLoader<CredentialStored>
// request data, per request
Operator, ClientAddr                                         // from events (R-019 note in R-022)
```

Anything missing gives INTERNAL at runtime. That path is tested, so a missing line fails safe but the feature doesn't work.

## Contract notes for the programme orchestrator

- **`deleteDevice` is two DELETEs.** It runs `clear_ssh`, then the device delete. K3 and R-004 say "one `DELETE`". The extra statement fixes the set vs delete deadlock (DEV-015 still holds through the cascade). The contract text needs a ruling.
- **K3 import row for `devices`.** It names `Vault, SshSecret, has_ssh, with_ssh, store_ssh, clear_ssh`. The code also uses `credentials::Secret`, to build an `SshSecret`, and `credentials::Error`, for error mapping. The import gate checks domains, so it passes, but K3 should name them.
- `Device` adds `created_at`/`updated_at` to the K3 struct. This is additive.

## Deferred (issue #1, R-007)

- **D-9.** `Site.devices` loads every device of the requested sites and pages them in memory. This is fine at Phase 1 scale. Revisit it if a site grows past a few hundred devices.
- From the final review (could):
  - Four id parsers with copied messages (C3).
  - Loader types in two places, the face and `devices::graphql` (C4).
  - K7 `UNAVAILABLE` unused: an unreachable database gives INTERNAL, as in the rest of the hub (C5).
  - The `deleteDevice` audit doesn't record that a secret was removed (C6, LOG-005).

## Open questions

- **Q-1.** `health` is in the hub but not in K1 (from WP-01). It affects the CON-002 test once `schema.rs` merges the parts.
- **Q-2.** `Device.ip` has no input field in K1 (D-8). Should `createDevice` set `ip` when `host` is an IP literal?
- A whitespace-only SSH password is rejected. This is a product choice; an empty passphrase means the client sends null.
