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

pub enum Method { Get }                      // the only variant: a provider request cannot express a write (writes: Section 13)
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

Later acquisition variants (OAuth pull, a stored plain token, collector push, file upload) are added to the `non_exhaustive` enums by contract change when a real source needs them (EXT-000). Nothing is built or named for them now. A write is never a new `Method` variant: `Method` stays `Get` only, and a write is a named `Change` (Section 13). A new `Change` variant is a contract change (operator, 2026-11-14; ruling number to be assigned; PRJ-027, OD-E14).

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
| `GET /connect/github/installed?installation_id&setup_action&state` | The cookie and S2 (the install link carries S2, and GitHub returns it on the Setup URL). Not trusted either way | Verifies the installation with the App JWT (Section 10a), then `establish`. A hit without the bound cookie gets a fixed page, changes nothing and asks GitHub nothing; the bound browser with a state that does not match, or with none, wakes the poll, which asks no sooner than one wait after its last ask (the poll interval, doubled after each unanswered ask or rate limit up to 5 minutes, reset by an answer) |

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

The installation is verified before `establish`: `GET /app/installations/{N}` with the App JWT must give 200, `app_id` equal to the attempt's, `account.id` equal to the converted `owner.id`, and not suspended; anything else is ignored. From `APP_CREATED` a task also asks `GET /app/installations?per_page=100` until the attempt leaves that phase or expires: every 5 seconds while GitHub answers, with the wait doubled after each reply that does not arrive, up to 5 minutes, and back to 5 seconds after an answer (Section 7a gives the wake rule), and takes the installation whose `account.id` is the owner's, so a closed tab does not stop the flow.

## 11. Scope

- **In now:** SRC-001; SRC-002 (the token-pull App-key variant only); SRC-003 to SRC-009; SRC-010 (through `write_reason`; GitHub states the PRJ-017 exception); SRC-011 and SRC-012; SRC-021 (poll part); KEY-001; KEY-002 (as `credentialStored`); ITG-001 to ITG-004 for the Sources list; the single-database claim and stale-owner rejection from issue 5.
- **Deferred:** OAuth pull and SRC-020; collector push; file upload and ITG-005; the webhook half of SRC-021 and WHK-*; ITG-007 (24 h reminder; needs reminders); the Outputs tab; issue 5's two-server identity, preferred server and failover.

## 12. Dependencies (K3)

`sources` imports `extensions`, `credentials` (the Section 6 functions only) and `events`; its `graphql.rs` also imports `access`. (operator, 2026-11-14; ruling number to be assigned): `registration/routes.rs` also imports `access` (`PublicOrigin`, `SameOrigin`, `identify`, and a cookie reader that `access` exports for it). `registration/` imports no other domain's `store`. No foundation or engine module imports an extension domain; wiring passes the intake in.

## 13. Live reads and named changes (operator, 2026-11-14; ruling number to be assigned; PRJ-027 to PRJ-035, OD-E14)

The Work, Code and History tabs of a project (K1 "projects: work, code and history") read GitHub live through the hub, and the first changes of OD-E14 reach GitHub as named changes. This section adds no table and no migration. It does not touch a sync run (Section 8) or the schedule (Section 10).

Facts about GitHub's REST API marked "to verify (C.7-n)" are not yet checked against GitHub's documentation; n is the item number of the verification list of the slice design. A builder confirms each one before the code that depends on it merges.

### 13.1 Read model

**Live reads, with a short cache in hub memory.** The hub reads GitHub when the operator asks, and keeps the raw answer in memory for a short time. Nothing of it is stored in PostgreSQL. Reasons: syncing four lists of every repository on a schedule would spend most of the installation's rate limit (5,000 requests an hour at the least, scaling with repositories and users up to 12,500) before paging; with no webhook a table would be up to 15 minutes old, and after a merge the operator expects the list to change at once; run and check states change every few seconds, so compare-before-write (SRC-012) gains nothing; Class C tables are not sealed, so private file content must not land in PostgreSQL; a cold cache refills on demand after a restart. GitHub's GraphQL API is not used: it is a POST with a free-form body, which the transport must not express. REST only.

Tables come with the webhook receiver (Section 2.12 of the plan), and only for what needs them: the list across projects (PRJ-036) and the weekly counts (PRJ-037), which are deferred.

**Cache.** Keyed by account, path and sorted query; it holds raw bodies.

| Answer | Fresh for |
| --- | --- |
| Lists of pull requests, issues, runs; comments | 30 s |
| One pull request (mergeability), one issue, one run | 15 s |
| Checks of a commit | 30 s; 10 min once nothing is pending |
| Jobs of a run | 15 s; 10 min once the run is completed |
| Commits, branches | 60 s |
| Releases, workflows, repository settings (merge methods) | 5 to 10 min |
| Directory | 60 s for a branch; 1 h for a 40-digit commit. Only a listing: GitHub answers a directory read of a file with its content, and that answer is never kept |
| File content | Never kept |

- **Fresh.** `fresh: true` skips the cache; the hub still reuses an answer less than 5 seconds old.
- **Stale.** A failed refresh serves the held answer for up to 1 hour with `ForgeRead.stale` true and fixed words in `ForgeRead.problem`. With nothing held the answer is `UNAVAILABLE`.
- **Bounds.** At most 2,000 entries and 64 MiB; no body over 512 KiB is kept; the least recently used entry goes first. The cache's core is pure and takes `now` as a parameter (CQ-021).
- **Forget.** A change drops the held answers of the paths it affects (13.6).
- **Headers.** Every request the transport sends to GitHub (sync, mint, live read and change) carries `X-GitHub-Api-Version: 2022-11-28` and `User-Agent: anima-hub`; every one but the manifest conversion (Section 7a, `Accept: application/json`) sends `Accept: application/vnd.github+json`. The hub uses no behaviour of a later version. The headers are the transport's, beside its `User-Agent`: the transport sends only to declared GitHub bases today, and a second source with another API would move them into its `Declaration` by contract change.
- **Budget.** The hub remembers the last `x-ratelimit-remaining` and `x-ratelimit-reset` it saw on a live answer (a read or a change) for each account, and assumes no total: GitHub's limit for an installation is at least 5,000 an hour and grows with its repositories and users, and a later answer may show more left than an earlier one, so the last answer wins. Live reads stop while 500 or fewer remain and the reset is ahead (`UNAVAILABLE`, "GitHub limits requests until HH:MM.", the reset time), which keeps the sync's share; with no reset header the stop lasts one minute. The operator's own change (a login or browser token) is not stopped by the reserve and may go down to 0. A change made with an API key stops at the reserve as a read does (`UNAVAILABLE`, the same words, nothing sent or recorded), and one key makes at most 20 changes a minute, counted in hub memory in fixed windows of a minute from its first (`UNAVAILABLE`, "This key makes at most 20 changes a minute. Try again in a minute.", nothing sent or recorded). The projects domain tells the engine which applies with a plain `Reserve` (`Spendable` or `Kept`); the engine learns nothing of tokens.
- **Request budget.** One GraphQL request may ask GitHub at most 60 times with an API key and 200 times with a login or browser token; every read counts, a cache hit included, so a document costs the same whatever the cache holds. Each read past it is refused before sending with `UNAVAILABLE`, "This request asked GitHub too many times. Ask for less at once.", and the rest of the request goes on. A page of 30 pull requests with mergeability and checks costs 91.
- **Limits.** A reply is a rate limit when its status is 429, or 403 with `x-ratelimit-remaining: 0`, or 403 with `retry-after`; it lasts until `retry-after` seconds, else the reset, else one minute, never more than a day. Live reads of the account then stop until it ends without sending. Any other 403 is "not permitted" and goes to the provider. (The sync keeps Section 7's rule.) A change's reply, a limit included, goes to the provider's `settle` as numbers in `Limit`, never as header text.
- **Pace.** For each account at most 2 live reads are in flight; its changes go one at a time, each at least 1 second after the previous one ended, on the hub clock. Another account is not held up. A request that waits more than 30 seconds for its turn fails (`UNAVAILABLE`). The turn is a semaphore permit; no lock is held across an await.
- **File content** (`codeFile`) is never cached, logged, put in an audit entry or an event. The file path is not in the process log (K7). A read writes no audit entry.

**Engine functions** (`engine/sources/live.rs`; K3):

```rust
pub async fn ask(s: &Scheduling, account: SourceAccountId, request: &Request, keep: Keep) -> Result<Reply, LiveFailure>;
pub async fn change(s: &Scheduling, account: SourceAccountId, change: &Change, reserve: Reserve) -> Result<Settled, LiveFailure>;
pub fn forget(transport: &Transport, account: SourceAccountId, path_prefix: &str);
pub async fn standing(conn: &mut PgConnection, accounts: &[SourceAccountId]) -> Result<Vec<(SourceAccountId, Standing)>, LiveFailure>;

pub struct Keep;      // Keep::for_seconds(n), Keep::never() (file content), .longer_when(n, settled: fn(&[u8]) -> bool), .only_when(holds: fn(&[u8]) -> bool), .fresh(bool)
pub struct Reply;     // response() -> Response<'_>, at(), stale(), problem() -> Option<LiveFailure>; its Debug shows no body
pub enum LiveFailure { NotFound, Paused, KeyRefused, Limited { until }, Unreachable, Unusable, Refused, NoChanges, Busy, Store }
pub enum Standing { Ready, NeedsReconnect, Paused, NotFound }   // from the stored row: paused first, then no key or an `auth` error
pub enum Reserve { Spendable, Kept }   // whether a change may spend the sync's reserve: the caller decides from who asked
```

`ask` returns any answer the source gave but a 401, a rate limit and a 5xx; only a 2xx answer is held. A read that fails with `Limited`, `Unreachable`, `Unusable`, `Busy` or `Store` serves the held answer, at most an hour old, with `stale` and the failure as `problem`. `forget` drops the answers whose path is the prefix or lies below it (`/repos/o/r/pulls` keeps `/repos/o/r/pulls2`). A paused account is refused (`Paused`) by `ask` and `change`.

`ask` uses `Request` (still `Method::Get`), the token of Section 7 and the transport's path and host checks. A live 401 forgets the token, as a sync's does, and writes nothing. The live path never claims an account, never writes `sources.accounts` or `sources.sync_runs`, and never changes `next_sync_at`, `failure_count` or the health. It shares only the HTTP client and `Tokens`.

### 13.2 Named changes (C.1)

`Method` stays `Get` only and `Request` is unchanged. A change is a value of one closed enum in `source-exchange` (`change.rs`):

```rust
pub enum Change {
    CreateIssue { repository: RepositoryName, title: Title, body: Option<Text>, labels: Vec<Label> },
    AddComment { repository: RepositoryName, number: Number, body: Text },
    SetIssueOpen { repository: RepositoryName, number: Number, open: bool, reason: Option<CloseReason> },
    MergePullRequest { repository: RepositoryName, number: Number, head: CommitSha, method: MergeMethod },
    RerunRun { repository: RepositoryName, run: RunId },
    RerunFailedJobs { repository: RepositoryName, run: RunId },
    CancelRun { repository: RepositoryName, run: RunId },
    DispatchWorkflow { repository: RepositoryName, workflow: WorkflowId, branch: BranchName, inputs: Vec<(InputName, InputValue)> },
    SetCommitStatus { repository: RepositoryName, sha: CommitSha, state: StatusState, context: StatusContext, description: Option<Text>, target_url: Option<String> },
}
pub enum Verb { Post, Put, Patch }                 // no Delete
impl Change { pub const fn verb(&self) -> Verb; pub const fn function(&self) -> &'static str; }
pub struct Rendered { pub path: String, pub body: Option<serde_json::Value> }
pub struct Changes { pub render: fn(&Change) -> Rendered, pub settle: fn(&Change, u16, Body<'_>) -> Settled }
pub enum Settled { Done(Receipt), NotFound, NotPermitted, NotMergeable, HeadMoved, Invalid, Gone, Refused }
```

- **Typed parts** (`parts.rs`). Each is a newtype with a private field and a checking `parse`: `RepositoryName` (the rule of `name::check`), `Number` and `RunId` (1 to 2^53), `CommitSha` (40 lower-case hex digits), `BranchName` (1 to 255 bytes, no `.` or `..` segment, no control character), `StatusContext` (`anima/[a-z0-9-]{1,40}`), and text with a length cap.
- **Verb fixed per variant.** `Change::verb` is a `const fn` in the exchange crate; a provider cannot choose it.
- **Rendered path fixed per variant.** `Declaration` gains `changes: Option<Changes>`; `None` for a read-only source. The provider's `render` builds the path with `format!` from the typed parts, always below `/repos/{owner}/{repo}/`, and the body with `serde_json`, never by joining strings. `settle` maps the status to `Settled` with fixed words and never quotes GitHub's reply.
- **One caller.** `sources::change` is the only caller of `Transport::change(declaration, change, token)`, which is `pub(in crate::engine::sources)` like `mint` and `convert`. It takes no path, verb, URL or `Rendered`: it calls the declaration's `render` itself, so its only inputs are a declared source and a checked `Change`. The path goes through `Request::get(path, &[])` and `transport.url(..)`, so the path and host checks of Section 7 apply. The body cap is 1 MiB (a 65,536-character `Text`, every character escaped, is far below it), with `Content-Type: application/json`; a body over the cap is refused before sending. A source whose declaration has `changes: None` is refused before sending (`NoChanges`).
- **Once.** A change whose connection failed before anything was sent is `Unreachable`. One that may have reached GitHub but got no whole answer (a timeout, a dropped connection) is `Settled::Unknown`, and the hub never sends it again. A 401 also drops the in-memory token, as a sync's does, and writes nothing.
- **Unreachable from a sync.** A caller outside the engine can only name a `Change` value. No `Intake` method returns one. The hub never repeats a change.
- GitHub's declaration names its changes: `changes` is `Some`, with the provider's `render` and `settle`, and `scopes` and `write_reason` state the changes it makes.

### 13.3 The functions (C.2)

All paths are below `/repos/{owner}/{repo}`. Endpoints, methods and body fields are those of GitHub's documentation for API version 2022-11-28 (checked 2026-11-14); success is 201 for an issue, a comment, both runs again and a status, 200 for open or close and for a merge, 202 for a cancel, and 204 for a start (200 when GitHub returns the run). Permissions, all among the App's 23 (Section 1): Issues write for the issue functions, Contents write alone for a merge, Actions write for the run controls and a start, Commit statuses write for a status. "Pre-read" is a fresh GET the hub makes first: never answered from the cache, not even stale, so a failed pre-read refuses the change and sends nothing. Only a start's workflow list is read as the cache holds it: it names the workflow, and GitHub refuses a start of one that is gone or turned off.

| Function | Endpoint | Pre-read | Repeat and idempotency |
| --- | --- | --- | --- |
| `createIssue` | `POST /issues` | none | Not idempotent. After a fault once sent, the issue may exist |
| `addIssueComment` | `POST /issues/{n}/comments` | none | As above |
| `setIssueOpen` | `PATCH /issues/{n}` | The issue: refuse a pull request (to verify, C.7-6); unchanged when already so | Idempotent |
| `mergePullRequest(s)` | `PUT /pulls/{n}/merge` with the method and `sha` | The pull request, up to 3 times 1 s apart while GitHub computes (to verify, C.7-4; GitHub documents 405 when it cannot merge and 409 when `sha` no longer matches, but no answer for an already merged pull request or a refused method, so 405, 409 and 422 are each a refusal) | The `sha` makes a second send safe; already merged returns unchanged |
| `rerunWorkflowRun` | `POST /actions/runs/{id}/rerun` | The run: status, head sha, fork (to verify, C.7-7) | A second send starts another attempt; the pre-read refuses a run that is not completed |
| `rerunFailedWorkflowJobs` | `POST /actions/runs/{id}/rerun-failed-jobs` | As above | As above |
| `cancelWorkflowRun` | `POST /actions/runs/{id}/cancel` | The run | Idempotent in effect |
| `dispatchWorkflow` | `POST /actions/workflows/{id}/dispatches` with `ref` and `inputs` (at most 25) | The workflow list (cached): active | Not idempotent: a second send starts a second run |
| `setCommitStatus` | `POST /statuses/{sha}` (at most 1,000 for a sha and context) | none | A second send adds an equal status |

**Merge rules.** The caller always names the method; there is no hub default, and `PullRequestConnection.mergeMethods` says which the repository allows (to verify, C.7-5). Not open, draft, `CONFLICT`, `BLOCKED` and `BEHIND` are refused before sending; `UNKNOWN` after three reads is `NOT_READY`; a head other than `expectedHeadSha` is `HEAD_MOVED`. `UNSTABLE` merges only alone and only with `allowFailingChecks: true`. The hub never bypasses a required check. A bulk merge is serial, in input order, each pre-read fresh just before its own merge, with a 90-second deadline for the whole request; the rest are `NOT_ATTEMPTED`.

**Forks (PRO-013).** A pull request from a fork merges one at a time, only with `forkConfirmed` equal to its head repository, only with a login or browser token, never in bulk (`FORK_REFUSED`), and never for a request that declares an AI agent (FORBIDDEN, as `openGithubConnect`). There is no MCP path for it. A run of a fork is not run again from Anima (`CONFLICT`); cancelling one is allowed because it stops code.

**Workflow inputs (PRJ-035).** GitHub's REST API does not give a workflow's inputs (to verify, C.7-8). The operator types name and value pairs, GitHub checks them, and the client links to the workflow file. The hub parses no YAML (operator, 2026-11-14; ruling number to be assigned).

### 13.4 Confirmations (C.4; PRJ-029, PRJ-033, PRJ-034)

The API binds each action to what the operator saw: `expectedHeadSha` on merge and on the three run controls.

| Action | Command line (prompt, or `--yes`) names | MCP `confirm` must equal |
| --- | --- | --- |
| Merge one | `owner/name#12`, short head sha, method | `owner/name#12` |
| Merge many | Every `owner/name#n` with its sha | Each entry carries its own `confirm` |
| Run again, run failed jobs again, cancel | Run number, workflow name, the commit | The run's 40-digit head sha |
| Start a workflow | Workflow path and branch | `<workflow path>@<branch>` |
| Commit status | none | none (the context is fenced to `anima/`) |
| Issue, comment, open or close | none | none |

### 13.5 Who may call what (K1, D-23)

Every read but `codeFile` is open to any token, a read key included. `codeFile` is closed to every API key and has no MCP tool: private code can hold a secret and the hub scans nothing. `createIssue`, `addIssueComment` and `setIssueOpen` take a write key. `mergePullRequest`, `mergePullRequests`, `rerunWorkflowRun`, `rerunFailedWorkflowJobs`, `cancelWorkflowRun`, `dispatchWorkflow` and `setCommitStatus` are login or browser token only: each makes code run with the repository's secrets, stops it, or can satisfy a required check (operator, 2026-11-14; ruling number to be assigned). The eight closed roots join `API_KEY_REFUSED_ROOTS`.

### 13.6 Audit, event and forgetting (C.5; PRJ-028)

- **When.** After GitHub's answer, in one transaction through `Trail`: every change that was sent gets an audit entry and an event, with its outcome (`done`, `refused`, `unknown`). A refusal before sending (validation, fork rule, not mergeable) records nothing. No transaction is held across the network call. The send, the forgetting and the record run in a task of their own, so a client that goes away after the change was sent still leaves its record.
- **If the record fails:** one more try, then `INTERNAL` and an error in the process log; it is not swallowed. When GitHub made the change the message is the fixed sentence "GitHub made the change, but the hub could not record it."; when no whole answer came back (`unknown`), "GitHub may have made the change, but the hub could not record it. Look before you try again."; otherwise the generic `INTERNAL` text. A bulk merge stops there with that error, as one merge would.
- **Entry.** Target `AuditTarget::Repository(RepositoryId)` (kind `repository`). The `change` record holds the function, the repository's full name, the target (`pull_request#12`, `issue#3`, `run#…`), the outcome, and as fits the head sha, method, run number, workflow, branch, context or issue number. Never a title, a body, a comment, an input value, file content or a reply of GitHub. Actor kind and token kind come from `Trail`.
- **Action.** `Create` (issue, comment, status), `Update` (open or close), `Start` (run again, run failed jobs again, start a workflow), `Cancel`, and the new `Merge`.
- **Event.** `Source::Hub`; Info for done, Warning for refused or unknown. Words with the project's slug and numbers only.
- **Forgotten afterwards:**

| Function | Dropped from the cache |
| --- | --- |
| Issue functions | The issue lists, that issue, its comments |
| Merge | Pull request lists, that pull request, commits and branches, checks of the base, runs |
| Run controls and start | Run lists, that run, its jobs |
| Commit status | Checks of that sha |

### 13.7 Failure words (C.6)

Fixed words, the GraphQL message; never a reply of GitHub (LOG-007):

| Words | Code |
| --- | --- |
| GitHub could not be reached. Nothing was changed. | `UNAVAILABLE` (fault before the request left) |
| GitHub did not answer. The change may have been made: look before you try again. | `UNAVAILABLE` (fault after sending) |
| GitHub limits requests until HH:MM. | `UNAVAILABLE` |
| The GitHub App is not permitted to do that. Check its permissions on GitHub. | `FORBIDDEN` (403 with no rate limit) |
| GitHub refused the key. Connect the account again. | `CONFLICT` |
| GitHub has no such issue / pull request / run / workflow. | `NOT_FOUND` |
| The pull request changed since you looked. Read it again. | `CONFLICT` |
| It cannot merge now. | `CONFLICT` |
| GitHub has not worked out whether it can merge. Try again. | `CONFLICT` |
| The repository does not allow that merge method. | `CONFLICT` |
| GitHub refused the branch or the inputs. | `VALIDATION` |
| Issues are off for this repository. | `CONFLICT` (410) |

### 13.8 The code browser

- **Path.** At most 1,024 bytes and 40 segments; no empty, `.` or `..` segment; no leading slash, backslash, control character or NUL. Each segment is percent-encoded by a pure function (to verify, C.7-11). The result passes `Request::get` and `Transport::url`, and is always appended to `/repos/{owner}/{repo}/contents/`.
- **Ref.** A `BranchName` or a `CommitSha`, sent only as the query value `ref`, never in the path.
- **Size.** Decided from the reply's `size` before decoding: over 1,000,000 bytes is `TOO_LARGE` with no content (GitHub's 1 MB limit for the default reply). The transport's 8 MiB cap stays the backstop.
- **Binary.** A NUL byte or invalid UTF-8 after decoding is `BINARY`, with no content. No lossy decoding.
- **Links.** GitHub answers a link to a file in the same repository with that file: it stays inside the repository, so it is shown, with the path GitHub read as `CodeFile.path`. A link object is shown as a `SYMLINK` entry and never followed; `codeFile` on it, or on a submodule, is `VALIDATION`.
- **Never interpreted.** The hub parses nothing in a file, and makes no promise that it holds no secret.
