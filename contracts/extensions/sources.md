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
pub struct Response<'a> { pub status: u16, pub body: Body<'a> }
pub enum Body<'a> { Read(&'a [u8]), TooLarge }   // TooLarge: past the engine's body cap; none of it kept
pub struct Problem { pub code: &'static str, pub subject: String, pub message: String }
pub struct Normalised<R> { pub records: Vec<R>, pub state: serde_json::Value, pub problems: Vec<Problem> }
```

### Registration (can the hub make the App?) (operator, 2026-11-14; ruling number to be assigned)

`AppKey` gains one field, `registration: Option<Registration>`. `None` means the provider cannot make its own App, and only the manual connect (`connectGithubApp`, `--from-file`) exists for it. GitHub's declaration sets `Some`.

```rust
pub struct Registration {
    pub web_base: &'static str,                                        // "https://github.com"
    pub name: fn(entropy: &[u8; 7]) -> String,                         // "anima-" + 10 lowercase base32 characters (16 of the 34 allowed)
    pub manifest: fn(&ManifestFacts<'_>) -> String,                    // JSON; pure
    pub form_path: fn(organisation: Option<&str>) -> Option<String>,   // None: not a login
    pub install_path: fn(slug: &str) -> String,
    pub settings_path: fn(owner: Option<&Owner>, slug: Option<&str>) -> String,   // where the operator deletes an App; the Apps list without a slug
    pub conversion_path: fn(code: &str) -> String,
    pub parse_converted: fn(&[u8]) -> Result<Converted, Problem>,
    pub installation_path: fn(id: &str) -> String,
    pub installations_path: &'static str,                              // "/app/installations"
    pub parse_installation: fn(&[u8]) -> Result<Installation, Problem>,
    pub parse_installations: fn(&[u8]) -> Result<Vec<Installation>, Problem>,
}
pub struct ManifestFacts<'a> { pub name: &'a str, pub origin: &'a str, pub webhook: Webhook }
pub enum Webhook { InactiveAtHub, Omitted, InactivePlaceholder }   // the hub picks one with a constant (Section 7a, live check)
pub struct Converted { pub app_id: u64, pub slug: String, pub owner: Owner, pub private_key_pem: String }   // no Debug
pub struct Owner { pub id: u64, pub login: String, pub organisation: bool }
pub struct Installation { pub id: u64, pub app_id: u64, pub account_id: u64, pub suspended: bool }
```

`parse_converted` reads only `id`, `slug`, `owner` and the private key. It never reads `client_secret` or `webhook_secret` into a value, and its `Problem` never quotes the reply. `Method` stays `Get` only: the one POST (the code conversion) is made by the transport, not described by a provider request.

The manifest asks for 23 repository permissions: `metadata` read and 22 others write (`contents`, `administration`, `actions`, `actions_variables`, `checks`, `deployments`, `environments`, `issues`, `pull_requests`, `statuses`, `workflows`, `secrets`, `repository_hooks`, `pages`, `discussions`, `repository_projects`, `security_events`, `vulnerability_alerts`, `dependabot_secrets`, `merge_queues`, `packages`, `secret_scanning_alerts`, `repository_custom_properties`). It sets `public: false`, `request_oauth_on_install: false`, no `callback_urls` and no `default_events`; `url`, `redirect_url` (`<origin>/connect/github/created`) and `setup_url` (`<origin>/connect/github/installed`) come from `ANIMA_PUBLIC_ORIGIN`. `hook_attributes` is `{ "url": "<origin>/connect/github/hook", "active": false }` for `InactiveAtHub`. `/connect/github/hook` is not routed.

Later acquisition variants (OAuth pull, a stored plain token, collector push, file upload) are added to the `non_exhaustive` enums by contract change when a real source needs them (EXT-000). Nothing is built or named for them now. A new `Method` variant (a write) is also a contract change.

## 2. Provider functions (SRC-006)

Each provider crate exposes exactly one constant and three pure functions:

```rust
pub const DECLARATION: Declaration;
pub fn begin(state: &Value) -> Value;                    // start of a run; keeps an unfinished phase (resume)
pub fn next_request(state: &Value) -> Option<Request>;   // None: the run is complete
pub fn normalise(request: &Request, response: &Response<'_>, state: &Value) -> Normalised<Record>;
```

No I/O, clock, randomness or logging. `state` is the provider's own slice only (Section 4). `normalise` of the same input twice gives equal output. The provider decides what `Body::TooLarge` means for its request: a reply the run cannot go on without (a listing) keeps the slice, so the engine fails the run; a reply about one record (one repository's files) is that record's problem, and the run moves on.

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
    /// Deletes this intake's records of the account (removeSourceAccount with deleteRecords true),
    /// except the operator's decisions, which it keeps without the account; returns how many it deleted.
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
- `transport.rs`: one `reqwest::Client` with redirects off, https only, 20 s timeout, 8 MiB body cap and `User-Agent: anima-hub`. Past the cap it stops reading, keeps none of the body and hands the normaliser `Body::TooLarge`; a mint reply past the cap is unusable.
  - The URL is always `base + request.path`. A path that does not start with `/` or that holds `://` or `@` is refused.
  - `Transport { client, base_override: Option<Url> }`: only tests set the override (a loopback stub). No environment variable.
  - 401 from the mint or from a request is `auth`. 403 or 429 with `x-ratelimit-remaining: 0` or `retry-after` is `rate_limit { until }`. Other non-2xx statuses go to the normaliser.

### Connect GitHub: the App is made from the hub (operator, 2026-11-14; ruling number to be assigned)

- `app_key.rs` splits into `AppSigner { app_id, pair }` (`read(app_id, pem)`, `token(now, random)`, the same limits) and `AppKey { signer, installation_id }`; `AppSigner::bind(installation_id)` makes the second. The flow needs an App JWT before an installation exists.
- `Transport::convert(url) -> Result<Answer, Fault>` is a POST with `Accept: application/json`, no `Authorization` and no body. It has the same visibility as `mint`.
- `acquisition/conversion.rs`: `convert(transport, declaration, code, now) -> Result<Converted, ConvertFailure>`, shaped like `mint`. The path goes through `Request::get(path, &[])` and `transport.url(api_base, ..)`, so the path check and the allowed hosts apply. `code` must match `[A-Za-z0-9_-]{1,200}` before it enters a path. `ConvertFailure` is `Refused` (4xx), `RateLimited`, `Unreachable`, `Unusable` or `NotRegistrable`. On `Unreachable` or a 5xx the hub tries once more after 2 seconds, then fails.
- App-JWT GETs: `GET /app/installations/{id}` and `GET /app/installations?per_page=100` go through `Transport::get` with the App JWT as the bearer. `id` must be 1 to 20 digits before it enters a path. The App JWT is never stored and has no `Debug`.
- The conversion reply body is `Content::Body(Zeroizing<..>)`, wiped on drop.

## 7a. The attempt to connect GitHub (operator, 2026-11-14; ruling number to be assigned)

One attempt exists at a time, in hub memory only (`Attempts`: one `Arc<Mutex<Option<Attempt>>>` and a broadcast sender, held in `Reach` like `Tokens`). The flow adds no table and no migration (K2 has no row for it). Reasons: one hub (Section 8), one operator, at most 55 minutes, and the only secret, the private key between "Create" and "Install", would otherwise need a sealed row with no account to belong to. Nothing secret is at rest until the sealed key at the end (Section 6). A hub restart drops the attempt; the audit entry "GitHub App created" holds the App ID and slug so the operator can still delete the App.

**Phases** (public): `WAITING_FOR_BROWSER`, `AT_GITHUB`, `APP_CREATED`, `CONNECTED`, `FAILED`, `CANCELLED`. `Finishing` is internal, taken under the lock, so the callback and the poll cannot both store. The work of `/created` (conversion, audit, `APP_CREATED`) and the store run in tasks of their own, so a browser that leaves does not stop them; a store that is dropped or panics returns the attempt to `APP_CREATED`, and one that holds `Finishing` past 2 minutes returns to `APP_CREATED` at the next step (an older store's outcome then counts only if it connected), after which expiry applies again (operator, 2026-11-14; ruling number to be assigned). Failures: `EXPIRED`, `EXCHANGE_REFUSED`, `GITHUB_UNREACHABLE`, `REPLY_UNUSABLE`, `KEY_REFUSED`.

**Limits.** An attempt lives 55 minutes from `startedAt`, in each phase (GitHub's code lives one hour). A ticket works once and for 5 minutes. A finished attempt stays readable for 10 minutes. A second start replaces an attempt that has made no App, and is `CONFLICT` once an App is made and not installed.

**Three values**: the ticket and the cookie are 32 random bytes each; S1 and S2 are HMAC-SHA-256 of a fixed label for each, keyed with the cookie, so one browser gets the same S1 and the same S2 on every ask. The hub keeps only SHA-256 hashes and compares in constant time:

- **Ticket**: travels from the client to the browser once, in `https://<hub>/connect/github?ticket=<T>`. It dies on first use or after 5 minutes. It is the only one-time value in a URL, and a path-only log never holds it (the request log records the route template, LOG-007).
- **Binding cookie** `__Host-anima-connect` (K7): never in a URL. `SameSite=Lax` so GitHub's redirect carries it. The hub session cookie `__Host-anima` cannot do this job: it is `SameSite=Strict` and needs an `Origin`, which a redirect from github.com does not have.
- **State** S1 (to GitHub with the manifest) and S2 (to GitHub with the install page): in the page the hub serves to the bound browser, in GitHub's URL and in the redirect back. Never in a GraphQL answer, an MCP result, a log or an audit entry.

**Routes** (fixed paths, no path parameters; behind the permitted-networks layer; before the web client fallback; each answer has `Cache-Control: no-store`, `Referrer-Policy: no-referrer`, `nosniff`, HSTS, COOP `same-origin` and its own CSP):

| Route | Authentication | Does |
|---|---|---|
| `GET /connect/github?ticket=T` | The ticket: hash equal, unused, under 5 minutes, phase `WAITING_FOR_BROWSER` or `AT_GITHUB` | Claims the attempt, sets the cookie, `303 /connect/github/start`. A bad ticket: 400 page, fixed words |
| `GET /connect/github` | None | A fixed confirm page with a form to `/open`. The same page whether or not an attempt exists |
| `POST /connect/github/open` | `SameOrigin` and `access::identify` with the `__Host-anima` browser session | Claims as above. 401 page without a session; 409 page when no attempt is open |
| `GET /connect/github/start` | Cookie hash equal; phase `AT_GITHUB` | Serves the browser's S1 (the same on every hit, so a second hit does not spoil the first) in a page that posts `manifest` to `https://github.com/settings/apps/new?state=S1` (or the organisation path). CSP `default-src 'none'; script-src 'sha256-<fixed script>'; style-src 'unsafe-inline'; form-action https://github.com; base-uri 'none'; frame-ancestors 'none'`. A "Continue to GitHub" button serves a browser without scripts |
| `GET /connect/github/created?code&state` | Cookie and S1 | Consumes S1, posts the conversion, keeps App facts and key in memory, `303` to GitHub's install page with S2. Phase `APP_CREATED`; the installation poll starts. Once the App is made: the spent S1 gets the same `303` again; anything else gets the fixed "finishing" page; neither changes the attempt |
| `GET /connect/github/installed?installation_id&setup_action&state` | The cookie; S2 if present. Not trusted either way | Verifies the installation with the App JWT (Section 10a), then `establish`. A hit without the bound cookie gets a fixed page, changes nothing and asks GitHub nothing; the bound browser with a state that does not match wakes the poll, which asks no sooner than one wait after its last ask (the poll interval, doubled after each unanswered ask or rate limit up to 5 minutes, reset by an answer) |

**What each attacker can do.**

- Learns the attempt id or `pageUrl`: nothing. No route takes the id; GraphQL needs a token.
- Learns the ticket before the operator's browser uses it, and is on the VPN: can bind their own browser and connect the hub to an App in their own GitHub account. This is the one real exposure. Bounds: 5 minutes, one use, a URL passed only from the client process to the browser and never to an agent or a log. The operator's browser then gets the "already used" page, and the client shows an owner login that is not theirs.
- Learns S1 or S2: nothing without the cookie.
- Cross-site, makes the operator's browser hit `/start`: the Lax cookie is sent, and the page holds the same S1 as before; the attempt is unchanged and the operator's return still exchanges.
- Cross-site, makes the operator's browser hit `/created`: the Lax cookie is sent, the state is unknown, the request is refused (in `AT_GITHUB`) or shown the "finishing" page (once the App is made), and the attempt is unchanged (no lockout).
- Cross-site, makes the operator's browser hit `/installed` with any `installation_id`: the hub asks GitHub with the App's own JWT; the worst case is an early check of the true installation.
- On the permitted networks without the cookie, hits `/installed` any number of times: a fixed page; the hub asks GitHub nothing.
- Cross-site POST to `/open`: refused by the Origin check; the Strict session cookie is not sent.

**Replay.** The ticket is single use. S1 is consumed before the exchange starts; its hash is kept until the attempt ends. A second hit on `/created` after success exchanges nothing: with the spent S1 it redirects to the install URL with the same S2, without it it shows the "finishing" page; neither changes the attempt. A second hit on `/installed` after `CONNECTED` shows the done page.

**Agent-started attempts** (operator, 2026-11-14): `openGithubConnect` is refused (FORBIDDEN) for a request that declares an AI agent, and no MCP tool returns the link. The link an agent gives the operator is `https://<hub>/connect/github` with no value in it; the operator continues from a browser signed in to Anima (`POST /open`) or from the desktop client ("Continue in browser").

**Secrets.** The private key moves at once into a `Zeroizing<String>` inside the attempt, which has no `Debug`, and is dropped on cancel, failure and expiry. The hub does not keep the client secret or the webhook secret: it never reads them out of the reply (the App requests no OAuth; the webhook is off, and a later receiver sets its own secret with `PATCH /app/hook/config`). Audit entries (target `Setting("sources.github.connect")`, then `SourceAccount`) hold only: started (label, organisation, actor), App created (`appId`, `slug`, `ownerLogin`), cancelled, and the existing "connected" entry.

**Default label**: the App owner's login, when `label` is null. **Organisations**: the API and the command line take an organisation; the graphical client offers the personal account only.

**GitHub's "Confirm access"** prompt (password or passkey) when the hub posts the form cannot be avoided; the client says so in one line.

**GitHub has no `DELETE /app`.** The hub cannot delete an App it made and no longer uses; `appSettingsUrl` (`https://github.com/settings/apps/<slug>/advanced`, or the organisation form; the Apps list without a slug) lets the operator do it.

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

## 10a. One storing path (operator, 2026-11-14; ruling number to be assigned)

`connect.rs` holds `pub async fn establish(s: &Scheduling, trail: Trail<'_>, input: Connection) -> Result<Established, Error>`, with `Established { id, created, consumer_on }`. It does check, prove (mint), store (`stored_key::to_secret` and `store_source_secret`), audit, commit, publish, and starts the first run when a consumer is on. `connectGithubApp` and the connect flow both call it; nothing else stores a source key. The flow's `Trail` is built with the operator and address captured at `startGithubConnect`. A mint that is unreachable, or a database error, leaves the attempt in `APP_CREATED` and the next poll tries again; a refused key is `FAILED` with `KEY_REFUSED`.

The installation is verified before `establish`: `GET /app/installations/{N}` with the App JWT must give 200, `app_id` equal to the attempt's, `account.id` equal to the converted `owner.id`, and not suspended; anything else is ignored. From `APP_CREATED` a task also asks `GET /app/installations?per_page=100` every 5 seconds until the attempt leaves that phase or expires, and takes the installation whose `account.id` is the owner's, so a closed tab does not stop the flow.

## 11. Scope

- **In now:** SRC-001; SRC-002 (the token-pull App-key variant only); SRC-003 to SRC-009; SRC-010 (through `write_reason`; GitHub states the PRJ-017 exception); SRC-011 and SRC-012; SRC-021 (poll part); KEY-001; KEY-002 (as `credentialStored`); ITG-001 to ITG-004 for the Sources list; the single-database claim and stale-owner rejection from issue 5.
- **Deferred:** OAuth pull and SRC-020; collector push; file upload and ITG-005; the webhook half of SRC-021 and WHK-*; ITG-007 (24 h reminder; needs reminders); the Outputs tab; issue 5's two-server identity, preferred server and failover.

## 12. Dependencies (K3)

`sources` imports `extensions`, `credentials` (the Section 6 functions only) and `events`; its `graphql.rs` also imports `access`. (operator, 2026-11-14; ruling number to be assigned): `registration/routes.rs` also imports `access` (`PublicOrigin`, `SameOrigin`, `identify`, and a cookie reader that `access` exports for it). `registration/` imports no other domain's `store`. No foundation or engine module imports an extension domain; wiring passes the intake in.
