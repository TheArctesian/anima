# K2 — Database ownership and migrations

Status: **frozen for Phase 3** (change only through a programme ruling, CON-001). Phase 2 additions are marked (Phase 2). Phase 3 additions are marked (Phase 3).

## Rules

- One PostgreSQL **schema per hub domain**. A domain's `store.rs` is the only code that names its schema (CQ-032). The import-graph and grep gates check this: the string `"<schema>.` may appear only under `crates/hub/src/<domain>/`.
- One migrations folder: `crates/hub/migrations/`, files `NNNN_<domain>_<what>.sql`. Each domain uses only its number range below, so parallel packages never collide (CON-004). sqlx applies any migration not yet applied, so a lower number added later still runs.
- Migrations are additive within Phase 1, Phase 2 and Phase 3, with one exception: the Q2-05 split (D-31) moves two tables to the schema `trust` (range 0920–0939). Do not edit a migration after its package is accepted; add a new one.
- Time-series tables are partitioned by day on their time column (DB-004). Retention drops whole partitions (DB-005). A partition helper (`create_daily_partitions`, `drop_partitions_before`) lives in the owning domain's store.
- Cross-domain references: a foreign key may point **into** a domain whose range is lower (it already exists). Log tables keep a copy of names instead of a foreign key, so they survive deletion.

## Ownership and ranges

| Range | Domain (package) | Schema | Tables |
| --- | --- | --- | --- |
| 0001–0099 | access (WP-05) | `access` | `operators`, `api_tokens`, `login_attempts`, `permitted_networks` |
| 0010–0029 (Phase 3) | access (WP-43) | `access` | `api_tokens.kind text not null default 'login' check (kind in ('login','browser','api_key'))`; `api_tokens.name text null` (required and unique for `api_key`, 1..64 characters); `api_tokens.display_prefix text null` (12 characters, `api_key` only); a check that `browser` and `login` rows have scope `write`; index `(kind, operator_id)`. Prune: expired `login` and `browser` rows at once, expired `api_key` rows 30 days after expiry |
| 0100–0199 | devices (WP-06) | `devices` | `sites`, `devices`, `device_tags`, `ssh_settings` |
| 0110–0199 (Phase 2) | devices (WP-21) | `devices` | `rdp_settings`, `vnc_settings` (device_id pk fk cascade, columns as K1, defaults in the schema, DB-011); `devices.mac_address macaddr null`, `devices.metrics_source text check in ('none','ssh') default 'none'`, `devices.transfer_upload`, `_download`, `_copy`, `_paste bool default true`; `sites.wake_relay_device_id uuid null fk -> devices.devices on delete set null` |
| 0120–0129 (Phase 3) | devices (WP-47) | `devices` | `devices.session_limit int null check (session_limit between 1 and 100)`; `devices.record_sessions bool not null default false` |
| 0200–0299 | credentials (WP-07) | `credentials` | `secrets` |
| 0210–0299 (Phase 2) | credentials (WP-21) | `credentials` | `secrets.kind` check adds `rdp_password`, `vnc_password` |
| 0300–0399 | checks (WP-09) | `checks` | `checks`, `heartbeats` (partitioned), `uptime_minute`, `uptime_hour`, `uptime_day` |
| 0310–0399 (Phase 2) | checks (WP-29) | `checks` | `checks.type` identifiers `docker`, `snmp`, `tailscale_ping`, `systemd`; SNMP secrets sealed in the config with context `checks.checks.snmp:<check_id>` |
| 0320–0349 (Phase 3) | checks (WP-46) | `checks` | `checks.type` adds `domain_expiry`; `checks.checks.domain_not_after timestamptz null` (beside `cert_not_after`); `checks.defaults` (`id bool pk default true check (id)`, `interval_seconds`, `retries`, `retry_interval_seconds`, `degraded_threshold_ms`, each with the schema default and the bounds of `checks.checks`), seeded with one row |
| 0400–0499 | maintenance (WP-09) | `maintenance` | `windows`, `window_targets` |
| 0500–0599 | events (WP-05) | `events` | `event_log` (partitioned), `audit_log` (partitioned) |
| 0503 (Phase 3) | events (WP-45, handoff) | `events` | `event_log_source_check` adds `notification` (as 0701 did for `metrics`) |
| 0600–0699 | sessions (WP-10) | `sessions` | `sessions`, `host_keys` |
| 0610–0649 (Phase 2) | sessions (WP-22) | `sessions` | `sessions.protocol` check `('ssh','rdp','vnc')`; `end_reason` adds `server_certificate_mismatch`; `server_certificates` (device_id pk fk cascade, fingerprint_sha256, subject null, not_after null, first_seen_at, accepted_by) |
| 0650–0699 (Phase 2) | sessions (WP-27) | `sessions` | None expected (transfers go to the audit log) |
| 0660–0679 (Phase 3) | sessions (WP-48) | `sessions` | `recordings` (`id uuid pk`, `session_id uuid not null unique fk -> sessions.sessions`, `device_id uuid null` (no foreign key, copied), `device_name text`, `protocol text`, `format text check (format in ('guacamole','asciicast'))`, `key_sealed bytea not null` (vault, context `sessions.recordings:<id>`), `started_at`, `ended_at null`, `size_bytes bigint not null default 0`, `complete bool not null default false`, `stopped_reason text null check (stopped_reason in ('size_limit','disk_full','write_failed'))`, `expires_at timestamptz not null`); index on `expires_at` |
| 0700–0799 (Phase 2) | metrics (WP-28) | `metrics` | `series` (id uuid pk, device_id fk cascade, kind, label, gpu_index null, unique (device_id, kind, label)); `samples` (series_id, at, value double precision; partitioned by day on `at`; pk (series_id, at); no fk, as heartbeats); `minute`, `hour`, `day` (series_id, bucket, avg, min, max, count; pk (series_id, bucket)); `gpus` (device_id fk cascade, index, name, vendor, has_vram, found_at; pk (device_id, index)); `discovery` (device_id pk fk cascade, found_at, boot_at) for MET-006 |
| 0800–0849 (Phase 2) | web_interfaces (WP-30) | `web_interfaces` | `interfaces` (id uuid pk, device_id fk cascade, label, url, open_in check in ('anima_tab','system_browser'), created_at); `pins` (interface_id pk fk cascade, fingerprint_sha256, accepted_by, accepted_at); `dismissed` (check_id pk fk -> checks.checks on delete cascade) |
| 0850–0899 (Phase 3) | notifications (WP-45) | `notifications` | `channels` (`id uuid pk`, `name text unique`, `kind text check (kind in ('webhook','ntfy','email','apprise'))`, `on_by_default bool default false`, `settings jsonb` (the non-secret fields, typed in Rust), `secrets bytea null` (sealed JSON of the kind's secret fields, context `notifications.channels:<id>`), `title_template text null`, `body_template text null`, `created_at`, `updated_at`); `check_channels` (`check_id fk -> checks.checks on delete cascade`, `channel_id fk on delete cascade`, pk both); `check_settings` (`check_id pk fk -> checks.checks on delete cascade`, `resend_every_beats int not null default 0 check (0..1000)`); `check_state` (`check_id pk fk -> checks.checks on delete cascade`, `notified_status text null`, `down_beats int not null default 0`, `expiry_marks jsonb not null default '[]'` (threshold and `notAfter` pairs sent)); `outbox` (`id uuid pk`, `channel_id fk on delete cascade`, `check_id uuid null`, `kind`, `payload jsonb` (rendered placeholder values, never a secret), `created_at`, `attempts int default 0`, `next_attempt_at`, `sent_at null`, `given_up_at null`); `deliveries` (`id uuid`, `channel_id uuid null`, `channel_name text`, `kind`, `at timestamptz`, `ok bool`, `message text null`; partitioned by day on `at`, no foreign key, as the logs); `settings` (one row: `certificate_days int[] default '{21,14,7}'`, `title_template`, `body_template`) |
| 0900–0919 (Phase 3) | retention (WP-46) | `retention` | `periods` (`subject text pk check (subject in ('heartbeats','metrics','events','recordings','deliveries'))`, `days int not null`, a per-subject bounds check as D-25), seeded with 30, 30, 90, 30, 30 |
| 0920–0939 (Phase 3) | trust (WP-41) | `trust` | `create schema trust`; `alter table sessions.host_keys set schema trust`; `alter table sessions.server_certificates set schema trust`. Metadata only (DB-010). Grants and the R-040 revocation move with the tables; migration 0001's default privileges give `anima_runtime` USAGE on the new schema |
| 0940–0999 | reserved | — | Phase 4 |

## Roles (R-016, LOG-006)

- **Owner role** (for example `anima_owner`, LOGIN, not superuser; CREATEROLE only when migration `0001` must create `anima_runtime` itself. The WP-04 deployment creates the roles first and gives the owner no CREATEROLE, R-021). Migrations run as this role, so it owns every schema, table, partition and function. Only the one-shot `migrate` step has its password.
- **Runtime group** `anima_runtime` (NOLOGIN), created by migration `0001`. The same migration runs `ALTER DEFAULT PRIVILEGES` for the owner role: USAGE on schemas; SELECT, INSERT, UPDATE and DELETE on tables; USAGE and SELECT on sequences; EXECUTE on functions. The hub's runtime login (for example `anima_hub`) is a member of this group and owns nothing.
- **Exceptions are explicit.** Migration `0500` revokes UPDATE, DELETE and TRUNCATE on `events.audit_log` and `events.event_log` from `anima_runtime` (WP-05 may use a narrower writer group instead). A new partition gets the same restriction.
- **Partitions.** The runtime role cannot create or drop a partition, because it does not own the parent. Each partitioned table has `SECURITY DEFINER` functions in its own schema, owned by the owner role, with `search_path = pg_catalog, pg_temp`: one creates a day partition, one drops partitions before a date. The drop function for `events.audit_log` refuses a date newer than its retention period (LOG-006). The domain's `store.rs` calls these functions.
- **Start check.** At start the hub refuses to serve, with a plain error (API-010), when its database login (`session_user`) is a superuser, differs from `current_user`, owns the database, schema `events` or any log table or partition, is a member of a role that can change, truncate, lock or reference the logs, or of `pg_execute_server_program`, `pg_write_server_files`, `pg_read_server_files` or `pg_maintain`, or would get write rights on new partitions through default privileges (R-023). The hub's database URL must not set `options=-c role=…`.
- One PostgreSQL cluster serves one Anima installation. `anima_runtime` is a cluster-wide role, so two installations in one cluster would share its grants (R-019).
- Tests run as the throwaway server's superuser (R-009). Each restriction above has a test that uses `SET ROLE anima_runtime`.

## Table outlines

Columns listed are the contract; a package may add columns to its own tables.

```text
access.operators          id uuid pk, username text unique, password_hash text (argon2id),
                          totp_secret_enc bytea null, created_at timestamptz
access.api_tokens         id uuid pk, operator_id uuid fk, token_hash bytea, scope text
                          check (scope in ('read','write')), expires_at, created_at, last_used_at
access.login_attempts     id uuid pk, at timestamptz, username text, client_addr inet, ok bool
access.permitted_networks id uuid pk, cidr cidr unique, label text

devices.sites             id uuid pk, name text unique, subnet cidr unique (VPN-001)
devices.devices           id uuid pk, site_id uuid fk, name text unique, host text,
                          ip inet null, kind text, os text check (os in
                          ('linux','macos','windows','other')), maintenance bool default false,
                          created_at, updated_at
devices.device_tags       device_id uuid fk on delete cascade, tag text, pk (device_id, tag)
devices.ssh_settings      device_id uuid pk fk on delete cascade, enabled bool, port int
                          default 22, username text, auth text check (auth in
                          ('password','private_key'))

credentials.secrets       id uuid pk, device_id uuid fk -> devices.devices on delete cascade,
                          kind text check (kind in ('ssh_password','ssh_private_key',
                          'ssh_passphrase')), nonce bytea, ciphertext bytea, key_id text,
                          created_at; unique (device_id, kind)

checks.checks             id uuid pk, device_id uuid fk on delete cascade, parent_id uuid null
                          fk (group), type text, name text, interval_seconds int default 60,
                          retries int default 3, retry_interval_seconds int default 20,
                          degraded_threshold_ms int default 1000, invert bool default false,
                          paused bool default false, config jsonb (typed per type in Rust),
                          operator_edited bool default false,
                          repair_tried_at timestamptz null (CHK-013)
checks.heartbeats         check_id uuid, at timestamptz, status text, response_time_ms int null,
                          message text null; partitioned by range (at); pk (check_id, at)
checks.uptime_minute|hour|day
                          check_id uuid, bucket timestamptz, up_count int, down_count int,
                          degraded_count int, maintenance_count int, avg_response_ms real;
                          pk (check_id, bucket)  (DB-008, DB-009)

maintenance.windows       id uuid pk, name text, kind text check (kind in ('manual','single',
                          'recurring_interval','recurring_weekday','recurring_day_of_month')),
                          timezone text, rule jsonb (typed in Rust, MW-004), active_manual bool
maintenance.window_targets window_id uuid fk on delete cascade, target_type text check
                          (target_type in ('device','check','tag')), target text

events.event_log          id uuid, at timestamptz, severity text, source text, device_id uuid
                          null, device_name text null, message text; partitioned by range (at)
events.audit_log          id uuid, at timestamptz, operator_id uuid, operator_name text, action
                          text, target_type text, target_id text, client_addr inet,
                          change jsonb; partitioned by range (at); the runtime role has
                          only SELECT and INSERT (LOG-006, R-016)

sessions.sessions         id uuid pk, device_id uuid null, device_name text, protocol text,
                          operator_id uuid, client_addr inet, started_at, ended_at null,
                          end_reason text null
sessions.host_keys        device_id uuid pk fk -> devices.devices on delete cascade,
                          algorithm text, fingerprint_sha256 text, first_seen_at,
                          accepted_by uuid
```

Note on `access.operators`: access (range 0001) cannot reference credentials (0200). TOTP secrets are therefore encrypted by access with the same vault key (via `credentials::Vault`, a pure crypto type with no tables — see `hub-domains.md`).

## Retention (Phase 1 defaults)

| Table | Raw | Aggregates |
| --- | --- | --- |
| `checks.heartbeats` | 30 days | minute 7 d, hour 90 d, day 2 y (DB-009) |
| `events.event_log` | 90 days | — |
| `events.audit_log` | 1 year | — |
| `access.login_attempts` | 90 days | — |
| `metrics.samples` (Phase 2) | 30 days | `minute` 7 d, `hour` 90 d, `day` 2 y (DB-009) |

(Phase 2) The metrics retention job removes samples whose series no longer exists (DEV-015, the R-004 pattern). Partition functions follow the "Partitions" rule above (`SECURITY DEFINER`, owner role).

Retention (Phase 3): the periods in `retention.periods` replace the fixed values of the K2 retention table for
`checks.heartbeats`, `metrics.samples`, `events.event_log`, `sessions.recordings` and
`notifications.deliveries`. The aggregates, `events.audit_log` (1 year, LOG-006) and `access.login_attempts`
(90 days) stay fixed. `notifications.deliveries` partitions follow the K2 "Partitions" rule (`SECURITY DEFINER`
functions owned by the owner role). Outbox rows are deleted 7 days after `sent_at` or `given_up_at`.
