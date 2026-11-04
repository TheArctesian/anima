# Programme state — Phase 1

Owner: programme orchestrator. Read by every agent. Rulings here override package plans (LOOP-003).

## Contracts (Wave 0)

| ID | File | State |
| --- | --- | --- |
| K1 | [contracts/schema.graphql](../../contracts/schema.graphql) | Frozen (operator approved 2026-11-03) |
| K2 | [contracts/database.md](../../contracts/database.md) | Frozen (operator approved 2026-11-03) |
| K3 | [contracts/hub-domains.md](../../contracts/hub-domains.md) | Frozen (operator approved 2026-11-03) |
| K4 | [contracts/tunnel.md](../../contracts/tunnel.md) | Frozen (operator approved 2026-11-03) |
| K5 | [contracts/tokens.md](../../contracts/tokens.md) | Frozen (operator approved 2026-11-03) |
| K6 | [contracts/fixtures/phase1.json](../../contracts/fixtures/phase1.json) | Frozen (operator approved 2026-11-03) |
| K7 | [contracts/conventions.md](../../contracts/conventions.md) | Frozen (operator approved 2026-11-03) |

The operator approved all seven contracts on 2026-11-03 (checkpoint 1). Only a ruling can change them now (CON-001).

## Packages

| Package | Wave | State | Branch | Session |
| --- | --- | --- | --- | --- |
| WP-01 Skeleton and gates | 0 | Building | `claude/wp-01-skeleton-212429` | `wp-01-skeleton` |
| WP-02 Tokens and fonts | 1 | Waiting (WP-01) | — | — |
| WP-03 VPN and network | 1 | Waiting (WP-01; operator checkpoint 2 for each VPS change) | — | — |
| WP-04 Deployment | 1 | Waiting (WP-01) | — | — |
| WP-05 Access and logs | 1 | Waiting (WP-01, K1–K3) | — | — |
| WP-06 Devices | 1 | Waiting (WP-01, K1–K3) | — | — |
| WP-07 Credentials | 1 | Waiting (WP-01, K3) | — | — |
| WP-08 guac-protocol | 1 | Waiting (WP-01) | — | — |
| WP-09 Checks and maintenance | 2 | Waiting (WP-05, WP-06) | — | — |
| WP-10 Sessions | 2 | Waiting (WP-05–WP-08) | — | — |
| WP-11 Client shell | 2 | Waiting (WP-02, K1) | — | — |
| WP-12 Devices page | 3 | Waiting (WP-11) | — | — |
| WP-13 Detail and form | 3 | Waiting (WP-11) | — | — |
| WP-14 SSH tab | 3 | Waiting (WP-11, K4) | — | — |
| WP-15 Settings page | 3 | Waiting (WP-11) | — | — |
| WP-16 Tauri shell | 3 | Waiting (WP-11) | — | — |
| WP-17 Integration and acceptance | 4 | Waiting (all) | — | — |

Integration branch: `phase-1`. Merge to `main` only after operator checkpoint 4.

## Rulings

- **R-001 (Wave 0).** Use **bun** as the JavaScript package manager and script runner, not pnpm (pnpm is not installed on the operator's machine). WP-01 updates the gate commands in `AGENTS.md` and plan Section 7 to `bun run --filter '*' lint`, `check` and `test`. Lockfile: `bun.lock`.
- **R-002 (Wave 0).** No shared `AppState` type. `main.rs` puts each dependency (`PgPool`, `events::Bus`, `credentials::Vault`, `Arc<dyn Clock>`, `Arc<Config>`) into the GraphQL schema data and passes them as explicit arguments to background tasks and routers. Reason: a shared struct that names domain types makes import cycles (K3).
- **R-003 (Wave 0).** WP-01 owns four leaf modules that every domain may import and that import no domain: `crates/hub/src/ids.rs` (all ID newtypes), `status.rs` (`Status` with `worst`), `clock.rs` (`Clock`, `SystemClock`, `FixedClock`), `config.rs` (`Config`). Reason: domains must not import each other only for an ID or the status enum (K3, K7).
- **R-004 (Wave 0).** `devices` never imports `checks`. Device deletion is one `DELETE` with foreign-key cascades; the checks retention job removes heartbeats of deleted checks (K3, DEV-015).

## Escalations

None open.
