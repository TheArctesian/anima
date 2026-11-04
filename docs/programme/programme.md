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
| WP-04 Deployment, stage A (hub) | 1 | Waiting (WP-01) | — | — |
| WP-04 Deployment, stage B (releases and self-update, R-006) | 3 | Waiting (WP-15, WP-16; operator sets secrets) | — | — |
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

- **R-005 (operator, 2026-11-03).** GitHub Actions runs only deployment, never gates or tests. All gates run locally with one command, `bun run gates`, before each review and before each merge into `phase-1`; the merge needs a green run on the merged result. WP-01 removes `ci.yml`. Spec CQ-010, TOK-002, MV-008, REPO-004 and acceptance item 15 are changed to match (checkpoint 3: requested by the operator).

- **R-006 (operator, 2026-11-03).** WP-04 also builds the self-updating desktop release pipeline (spec Section 3.9, UPD-001–012), as stage B, after WP-15 and WP-16 are accepted. Operator decisions: host on public GitHub Releases; version `0.1.<run number>`; macOS universal and Linux AppImage only; ad-hoc macOS signing with automatic Developer ID signing when all Apple secrets exist; `allowDowngrades: true`. Under R-005, the release workflow is the only GitHub Actions workflow and does not run on pull requests. Stage B may edit, after their owners are accepted and one at a time: in WP-16's `apps/desktop/src-tauri/`, the updater and process plugin lines in `Cargo.toml` and `lib.rs`, the `bundle.createUpdaterArtifacts` and `plugins.updater` keys in `tauri.conf.json`, and the "Check for Updates…" menu item; one mount line for the banner in WP-11's root layout; the Updates row in WP-15's Settings page. The operator runs `tauri signer generate` and `gh secret set TAURI_SIGNING_PRIVATE_KEY` personally (UPD-012). Agents never read the key file.

- **R-007 (operator, 2026-11-03).** Work that is decided but left for later (for example the TestFlight release) goes on GitHub issue [#1 Deferred work](https://github.com/TheArctesian/anima/issues/1) as a checklist item, with the spec IDs. A package orchestrator that defers something asks the programme orchestrator to add it there; it does not leave it only in a package plan or a report.

## Escalations

None open.
