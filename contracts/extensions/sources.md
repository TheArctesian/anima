# K-E1 — Sources (engine)

Status: new (operator, 2026-11-13; ruling number to be assigned). Proposed for the path to Projects; it is not part of the Phase 3 freeze. Change it only through a programme ruling (CON-001). Requirement IDs: SRC-*, ITG-*, KEY-*, PRJ-017, PRJ-018 of `docs/extensions-plan.md`; EXT-000 to EXT-012 for the extension switch.

A source is a place the hub reads records from (GitHub first). A source has one provider crate with pure code, and one or more intakes in the extensions that use it. The engine domain `crates/hub/src/engine/sources/` owns accounts, credentials by reference, schedule, runs and health. Owner: engine (`crates/hub/src/engine/**`, `crates/providers/exchange/**`, `$lib/sources/**`).

## 1. Shared types (crate `source-exchange`, `crates/providers/exchange`)

A provider must not import the hub (REPO-E-002), so the types they share live in this crate. Dependencies: `serde`, `serde_json`, `chrono` only.

```rust
pub struct Declaration {
    pub key: &'static str,                  // "github" (SRC-001)
    pub name: &'static str,                 // "GitHub"
    pub acquisition: Acquisition,
    pub scopes: &'static [&'static str],    // what the connection can do, shown as text
    pub write_reason: Option<&'static str>, // None: read-only scopes; Some(reason) otherwise (SRC-010)
    pub gives: &'static [&'static str],     // "Repositories", "Deployment records"
    pub poll_interval_seconds: u32,         // at least 900 (SRC-021)
    pub api_base: &'static str,             // "https://api.github.com"
}
#[non_exhaustive] pub enum Acquisition { TokenPull(Minting) }
#[non_exhaustive] pub enum Minting { AppKey(AppKey) }
pub struct AppKey {                          // the provider-specific part of "token pull with a hub-held App key"
    pub mint_path: fn(installation: &str) -> String,
    pub parse_minted: fn(body: &[u8]) -> Result<Minted, Problem>,
}
pub struct Minted { pub token: String, pub expires_at: DateTime<Utc> }

pub enum Method { Get }                      // the only variant: a provider cannot express a write
pub struct Request { pub method: Method, pub path: String, pub query: Vec<(String, String)> }
pub struct Response<'a> { pub status: u16, pub body: &'a [u8] }
pub struct Problem { pub code: &'static str, pub subject: String, pub message: String }
pub struct Normalised<R> { pub records: Vec<R>, pub state: serde_json::Value, pub problems: Vec<Problem> }
```

Later acquisition variants (OAuth pull, a stored plain token, collector push, file upload) are added to the `non_exhaustive` enums by contract change when a real source needs them (EXT-000). Nothing is built or named for them now. A new `Method` variant (a write) is also a contract change.

## 2. Provider functions (SRC-006)

Each provider crate exposes exactly one constant and three pure functions:

```rust
pub const DECLARATION: Declaration;
pub fn begin(state: &Value) -> Value;                    // start of a run; keeps an unfinished phase (resume)
pub fn next_request(state: &Value) -> Option<Request>;   // None: the run is complete
pub fn normalise(request: &Request, response: &Response<'_>, state: &Value) -> Normalised<Record>;
```

No I/O, clock, randomness or logging. `state` is the provider's own slice only (Section 4). `normalise` of the same input twice gives equal output.

## 3. Intake (hub side, `crates/hub/src/engine/sources/intake.rs`)

The engine cannot import an extension domain (EXT-005), so each extension hands the engine an intake through wiring:

```rust
#[async_trait] pub trait Intake: Send + Sync {
    fn source(&self) -> &'static str;          // Declaration.key
    fn extension(&self) -> extensions::Key;    // the consumer; the intake is used only while this is on
    fn slice_key(&self) -> &'static str;       // "github.projects"
    fn begin(&self, slice: &Value) -> Value;
    fn next_request(&self, slice: &Value) -> Option<Request>;
    /// Normalises one response and stores its records in `conn`, the engine's transaction.
    async fn take(&self, conn: &mut PgConnection, page: Page<'_>) -> Result<Taken, IntakeError>;
    /// Deletes this intake's records of the account (removeSourceAccount with deleteRecords true).
    async fn forget(&self, conn: &mut PgConnection, account: SourceAccountId) -> Result<u64, IntakeError>;
}
pub struct Page<'a> { pub account: SourceAccountId, pub request: &'a Request, pub response: Response<'a>, pub slice: &'a Value, pub now: DateTime<Utc> }
pub struct Taken { pub slice: Value, pub counts: Counts, pub problems: Vec<Problem> }
pub struct Counts { pub inserted: u32, pub changed: u32, pub unchanged: u32, pub removed: u32 }
pub struct Registry { /* Vec<Declaration>, Vec<Arc<dyn Intake>> */ }   // built once in wiring, put in the schema data
```

A source may have several intakes (GitHub: Projects now, People later). Setting one consumer off leaves the others running.

Obligations on an intake: SRC-011 (keep the provider's original record beside the canonical fields) and SRC-012 (compare before writing, so a second run over unchanged data reports only `unchanged`). A page replayed after a failure must be safe for the same reason.

## 4. Sync state (SRC-004)

`sources.accounts.sync_state` is one JSON object. Its top-level keys are `Intake::slice_key()` values. The engine gives an intake only `sync_state[slice_key]` (or `{}`) and writes back only that key. Engine bookkeeping is in columns, never in the JSON. Every slice has `"v": <int>`; a provider that reads an unknown `v` starts from `{}`.

## 5. Tables (schema `sources`, migrations 2000–2019)

```text
sources.accounts   id uuid pk, source text not null, label text not null (1..64, cleaned),
                   instance_key text not null, unique (source, instance_key),
                   credential_id uuid null references credentials.source_secrets(id),   -- SRC-003
                   sync_state jsonb not null default '{}',
                   last_started_at timestamptz null, last_sync_at timestamptz null (last success),
                   last_error_kind text null check in ('auth','rate_limit','provider','network','store'),
                   last_error text null (fixed words only), failure_count int not null default 0,
                   next_sync_at timestamptz not null, paused_at timestamptz null,
                   run_generation bigint not null default 0, run_until timestamptz null,
                   created_at timestamptz not null
sources.sync_runs  id uuid pk, account_id uuid fk cascade, trigger text check in ('connect','schedule','manual'),
                   started_at, ended_at null, outcome text null check in ('ok','failed','stopped'),
                   inserted, changed, unchanged, removed int not null default 0,
                   problems jsonb not null default '[]', error text null; index (account_id, started_at desc)
```

When a run ends, the store deletes that account's runs beyond the newest 200.

## 6. Credential (SRC-003, SRC-008, KEY-001, KEY-002, PRJ-018)

The account holds a credential reference (`credential_id`), never the value. The value is a sealed row in `credentials.source_secrets` (K2 migration 0221; functions in K3). The secret is not a column of `sources.accounts`.

- AEAD context: `credentials.source_secrets:<account_id>`.
- Plaintext for a GitHub App key: JSON `{"v":1,"app_id","installation_id","private_key_pem"}`, built in a zeroizing buffer as `notifications/sealing.rs` does.
- When the engine opens the secret it checks `app_id/installation_id == accounts.instance_key`. A mismatch counts as an `auth` error, so a database writer cannot point the key at another installation.
- `load_source_secret` has one caller: the token minter. No GraphQL field returns the value; `SourceAccount.credentialStored` is a boolean (KEY-002).

## 7. Acquisition now: token pull with an App key (`engine/sources/acquisition/`)

- `app_key.rs` (pure): parse the PEM with `rustls-pki-types`; build an RS256 JWT with `ring::signature::RsaKeyPair` (`iss` = App ID, `iat` = now − 60 s, `exp` = now + 540 s); refuse a key under 2048 bits and a PEM over 16 KiB.
- `tokens.rs`: `Tokens` is an in-memory map `SourceAccountId -> (Secret, expires_at)`. It mints with `POST {api_base}{mint_path(installation)}` and mints again when under 5 minutes remain. The token is never written to the database or a log, has no `Debug`, and its header value is marked sensitive.
- `transport.rs`: one `reqwest::Client` with redirects off, https only, 20 s timeout, 8 MiB body cap and `User-Agent: anima-hub`.
  - The URL is always `base + request.path`. A path that does not start with `/` or that holds `://` or `@` is refused.
  - `Transport { client, base_override: Option<Url> }`: only tests set the override (a loopback stub). No environment variable.
  - 401 from the mint or from a request is `auth`. 403 or 429 with `x-ratelimit-remaining: 0` or `retry-after` is `rate_limit { until }`. Other non-2xx statuses go to the normaliser.

## 8. One run (`engine/sources/run.rs`; the order is fixed for SRC-007 and SRC-012)

1. **Claim:** `UPDATE accounts SET run_generation = run_generation + 1, run_until = now + 10 min, last_started_at = now WHERE id = $1 AND (run_until IS NULL OR run_until < now) RETURNING run_generation`. No row means another run holds the account: skip. Insert the `sync_runs` row.
2. For each intake of the source whose extension is on: `slice = intake.begin(slice)`, then loop:
   - read `extensions::on_keys` again; if the consumer is now off, stop with outcome `stopped`;
   - `next_request`; `None` ends the loop;
   - fetch;
   - **one transaction:** `SELECT … WHERE id = $1 AND run_generation = $g FOR UPDATE` (no row: `LostClaim`, write nothing, stop); `intake.take(conn, page)`; write the slice key; add the counts to the run row; extend `run_until`; commit.
   - The slice (the cursor) moves only in the transaction that stored the page.
3. At most 2,000 requests per run. At the cap the outcome is `stopped` and the next run resumes.
4. **End** (guarded by the generation): `run_until = NULL`. On success: `last_sync_at = now`, `failure_count = 0`, error fields null. On failure: `failure_count += 1`, error kind and fixed words. Set `next_sync_at` from `schedule::next`.
5. One Warning event (`events::Source::Hub`) is written only when health changes into STALLED or NEEDS_RECONNECT.

The one PostgreSQL is the authoritative claim boundary and the generation is the fencing value. There is one hub. Two trusted servers, a preferred server and failover are deferred; GitHub issue 5 stays open for them.

## 9. Health (SRC-009), pure, `engine/sources/health.rs`

```rust
pub struct HealthFacts { pub paused: bool, pub consumer_on: bool, pub last_error_kind: Option<ErrorKind>,
    pub last_started_at: Option<DateTime<Utc>>, pub last_sync_at: Option<DateTime<Utc>>,
    pub failure_count: u32, pub interval: Duration }
pub fn health(facts: &HealthFacts, now: DateTime<Utc>) -> Health
```

First match wins:

1. `paused` or not `consumer_on`: PAUSED.
2. `last_error_kind == Auth`: NEEDS_RECONNECT.
3. No `last_sync_at`: `failure_count >= 3` is STALLED; `last_started_at` set is STARTING; otherwise NEVER_SYNCED.
4. `failure_count >= 3` or `now - last_sync_at > 3 × interval`: STALLED.
5. Otherwise OK.

`pausedBy` is OPERATOR when `paused`, otherwise EXTENSION_OFF.

## 10. Schedule (SRC-021), `engine/sources/schedule.rs` (pure) and `spawn_scheduler`

- `sources::spawn_scheduler(Scheduling { pool, bus, clock, vault, registry, transport, feed })` ticks every 60 s. It reads accounts with `paused_at IS NULL AND next_sync_at <= now AND (run_until IS NULL OR run_until < now)`, drops in Rust those with no intake whose extension is on (their `next_sync_at` is not touched), and runs the rest, at most 4 at a time.
- `schedule::next(now, interval, failure_count, rate_limited_until)`: success is `now + interval`; failure is `now + min(interval × 2^failure_count, 6 h)`; a rate limit is the larger of that and `until`.
- GitHub's interval is 900 s. It becomes the slow repair poll when a webhook receiver exists; the receiver is not specified here.
- `syncSourceAccount` and `connectGithubApp` run the same `run` in a spawned task, with trigger `manual` and `connect`.

## 11. Scope

- **In now:** SRC-001; SRC-002 (the token-pull App-key variant only); SRC-003 to SRC-009; SRC-010 (through `write_reason`; GitHub states the PRJ-017 exception); SRC-011 and SRC-012; SRC-021 (poll part); KEY-001; KEY-002 (as `credentialStored`); ITG-001 to ITG-004 for the Sources list; the single-database claim and stale-owner rejection from issue 5.
- **Deferred:** OAuth pull and SRC-020; collector push; file upload and ITG-005; the webhook half of SRC-021 and WHK-*; ITG-007 (24 h reminder; needs reminders); the Outputs tab; issue 5's two-server identity, preferred server and failover.

## 12. Dependencies (K3)

`sources` imports `extensions`, `credentials` (the Section 6 functions only) and `events`; its `graphql.rs` also imports `access`. No foundation or engine module imports an extension domain; wiring passes the intake in.
