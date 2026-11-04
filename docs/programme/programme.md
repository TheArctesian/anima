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
| WP-01 Skeleton and gates | 0 | Accepted, merged into `phase-1` (gates 12/12) | `claude/wp-01-skeleton-212429` | `wp-01-skeleton` |
| WP-02 Tokens and fonts | 1 | Accepted, merged into `phase-1` (965feb2) | `claude/wp-02-tokens-327209` | `wp-02-tokens` |
| WP-03 VPN and network | 1 | Building (prepares files only; operator confirms each VPS, Headscale and Cloudflare change) | own worktree from `phase-1` | `wp-03-vpn-and-network` |
| WP-04 Deployment, stage A (hub) | 1 | Building (prepares files only; operator confirms each host and Cloudflare change) | own worktree from `phase-1` | `wp-04-hub-deploy` |
| WP-04 Deployment, stage B (releases and self-update, R-006) | 3 | Waiting (WP-15, WP-16; operator sets secrets) | — | — |
| WP-05 Access and logs | 1 | Building; T1 and T7 merged early into `phase-1` (2581982) | own worktree from `phase-1` | `wp-05-access` |
| WP-06 Devices | 1 | Building; T1 migrations merged early into `phase-1` (668c898) | own worktree from `phase-1` | `wp-06-devices` |
| WP-07 Credentials | 1 | Accepted, merged into `phase-1` (3f1f830) | own worktree from `phase-1` | `wp-07-credentials` |
| WP-08 guac-protocol | 1 | Accepted, merged into `phase-1` (c19ad06) | own worktree from `phase-1` | `wp-08-guac` |
| WP-09 Checks and maintenance | 2 | Waiting (WP-05, WP-06) | — | — |
| WP-10 Sessions | 2 | Waiting (WP-05–WP-08) | — | — |
| WP-11 Client shell | 2 | Building | own worktree from `phase-1` | `wp-11-client-shell` |
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

- **R-008 (operator, 2026-11-03).** Use the domain `anima.ing` (DNS on Cloudflare) as in spec Section 4.2.1: `hs.anima.ing` for Headscale, a public `hub.anima.ing` record that points to the hub's VPN address, `updates.anima.ing` as a redirect to the GitHub release feed, and VPN-only names under `net.anima.ing`. The hub gets a Let's Encrypt certificate by DNS-01. WP-03 owns the DNS records and the Headscale names; WP-04 owns the hub certificate (stage A) and the update redirect (stage B). Every Cloudflare change needs the operator's confirmation in chat (DNS-008); agents prepare the records in `deploy/dns/` and do not apply them.

- **R-009 (Wave 1, from WP-07).** Database tests always run in `bun run gates`. The `cargo-test` gate starts a throwaway PostgreSQL 17 on a Unix socket and sets `DATABASE_URL` (`quality/bin/with-postgres.ts`); a set `DATABASE_URL` is used as is. A missing PostgreSQL fails the gate. Tests must not skip when no database is present (CQ-024). A domain's tests must not create tables in another domain's schema, not even a stub (CQ-032): a store whose migration has a foreign key into a domain that is not on `phase-1` yet waits until that domain's migration lands. WP-06 delivers its `devices` migration first so that WP-07 (and later WP-09, WP-10) can build on it.

- **R-010 (Wave 1, from WP-07; changes K3).** (a) `devices` owns the GraphQL mutations `setSshCredential` and `clearSshCredential` and may call `credentials::{Vault, SshSecret, has_ssh, store_ssh, clear_ssh}`. `clear_ssh(conn, device) -> Result<bool, Error>` is added to `credentials`. (b) `Vault::seal` and `Vault::open` take a `context: &[u8]` that is bound as AEAD associated data, with the fixed context strings in K3, so a sealed value moved to another row does not open. Affects WP-05 (TOTP), WP-06, WP-07.

- **R-011 (Wave 1, from WP-06; changes K3).** (a) A domain's `graphql.rs` may import extra domains listed under `graphql` in `quality/hub-modules.json`, only to resolve K1 fields that join domains; never a `store`, and these edges do not count toward cycles. `devices/graphql.rs` may import `checks` and `sessions`. (b) WP-06 builds `Device` and `SshSettings` with the fields devices owns, `DeviceFilter` name/tags/siteId and `DeviceSort.NAME`; it does not declare the fields that need other domains. WP-09 adds `Device.status`, `checks`, `uptime`, `DeviceFilter.status`, `DeviceSort.STATUS`/`UPTIME` and the `addDefaultCheck` behaviour to `devices/graphql.rs` after WP-06 is accepted; WP-10 adds `SshSettings.hostKey`. One owner at a time (PKG-001). (c) Until WP-17, the schema contract check (CON-002) requires only that every hub field matches K1; WP-17 checks that K1 is complete.

- **R-012 (Wave 1, from WP-06; changes K3).** The K1 scalars `IpAddr` and `Cidr` live in the leaf module `crates/hub/src/scalars.rs` (programme-owned, like R-003). `Cidr` rejects host bits. `DateTime` and `JSON` are async-graphql's built-in chrono and `Json<T>` scalars. A domain must not define a scalar with a K1 name. Storage conversion (for example to `ipnet::IpNet` for sqlx) stays in each domain's `store.rs`.

- **R-013 (Wave 1, from WP-02; changes K5).** K5 adds `projection-deceleration` (0.998) and `rubberband-constant` (0.55), TypeScript only, in a `gesture` object (MOT-032, MOT-033). The TypeScript `reducedMotion` and `breakpoint` objects are a correct reading of K5: they carry values, not new names.

- **R-014 (Wave 1, from WP-06; changes K3).** K1's `PageInfo` is `paging::PageEnd` (GraphQL name `PageInfo`) in the leaf module `crates/hub/src/paging.rs`; no domain defines its own or uses async-graphql's built-in connection `PageInfo`. Data loaders (API-005): each domain exposes a constructor in its `graphql.rs` (for example `devices::graphql::site_loader(pool)`), and `schema.rs` registers it with `.data(..)` as a wiring handoff. The loaders have no cache, so one schema-wide instance is safe.
- **R-015 (Wave 1, from WP-05; changes the limits gate).** A Rust module face `name.rs` that has a sibling folder `name/` is one unit with that folder (CQ-004). It does not count as a file in its parent folder for REPO-005. So the hub crate root counts `lib.rs`, `main.rs`, `schema.rs`, the leaf modules and the face of each domain that has no folder yet. Do not move the leaf modules; K3 paths do not change. A domain that grows past one file gets its folder.
- **R-016 (Wave 1, from WP-05; changes K2 and spec 3.6).** LOG-006 needs a runtime role that cannot change the audit log, so migrations and serving use different roles (K2 "Roles"). The hub binary gets a `migrate` subcommand that applies the migrations and exits. Compose runs it as a one-shot service with the owner role's secret, and the hub service starts only after it completes; the hub service has only the runtime login's secret. Owners: WP-05 writes migration `0001`'s runtime group and default privileges, migration `0500`'s restrictions and partition functions, and `events::check_runtime_role` (the start check); `main.rs` gets the `migrate` subcommand and the start-check call as wiring handoffs. WP-04 stage A writes the two roles, the two secrets and the one-shot service. WP-09 adds the partition functions for `checks.heartbeats`. Merged migrations do not change, because the default privileges cover tables made later in the same run.
- **R-017 (Wave 1, from WP-06; changes K3).** `credentials` adds `with_ssh(db, devices: &[DeviceId]) -> Result<HashSet<DeviceId>, Error>`, one query with `device_id = ANY($1)`, so `SshSettings.credentialStored` batches through a loader (API-005). `has_ssh` stays for single-device callers. WP-07 owns it, with a test that an empty slice returns an empty set without a query error; WP-06 uses it from a `CredentialStored` loader.
- **R-018 (Wave 1, from WP-08; changes K4 wording).** Q-1: the hub answers guacd's `args` with `VERSION_1_5_0`, because echoing a newer version would claim features the hub does not have. Q-2: a relayed `size` may omit DPI (guacamole-common-js 1.5.5 and 1.6.0 do); when present it must be 48..480. No code changes; WP-10 follows the new wording.
- **R-019 (Wave 1, from WP-05; changes K2, K3 and test practice).** (a) K3 placed `Operator`, `ClientAddr` and `HeartbeatSummary` in access and checks, but `events` uses them and imports no domain. They are defined in `events`; access re-exports `Operator` and `ClientAddr`, and checks uses `events::HeartbeatSummary`. K3 lists `create_partitions_ahead`, `drop_expired_partitions` and `check_runtime_role` as wiring-only. (b) K2: one PostgreSQL cluster per installation, because `anima_runtime` is cluster-wide. (c) No release-build gate for API-007: `harden` takes the production flag as a parameter and the tests cover both values; the one `cfg!(debug_assertions)` line is checked in review. (d) Agents and reviewers leave `DATABASE_URL` unset, so each run gets its own with-postgres server. sqlx names test databases by test path, so two worktrees that share one server destroy each other's databases (the 23505 and 3D000 failures).
- **R-020 (Wave 1, from WP-11; changes K7).** Token transport is in K7 "Authentication transport": a bearer header on HTTP, the `connection_init` payload on `graphql-transport-ws`, close code 4403 when it is missing or not valid. WP-05 T3 implements the hub side. Until the hub writes its own SDL file, WP-11 generates the client's TypeScript types (API-008) from `contracts/schema.graphql`; WP-17 switches the source when the CON-002 check lands.

## Escalations

None open.
