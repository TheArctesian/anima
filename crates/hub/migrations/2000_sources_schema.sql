-- The sources domain's tables (K-E8 range 2000-2019; K-E1 Section 5; SRC-003, SRC-004,
-- SRC-005, SRC-009, SRC-021).
--
-- An account holds a reference to its sealed credential (`credential_id`), never the value
-- (SRC-003, PRJ-018); the installation token it mints lives in hub memory only. Engine
-- bookkeeping is in columns; `sync_state` holds only the intakes' slices, keyed by slice key
-- (SRC-004). `run_generation` is the fencing value of the claim: a run writes only while the
-- generation it claimed is still the account's (K-E1 Section 8). `last_error` holds fixed
-- words only, never a provider reply, a token or a key (LOG-007).
--
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles").
create schema sources;

create table sources.accounts (
    id              uuid        primary key,
    source          text        not null check (source ~ '^[a-z][a-z0-9_]*$' and length(source) <= 40),
    label           text        not null check (length(label) between 1 and 64),
    instance_key    text        not null check (length(instance_key) between 1 and 128),
    credential_id   uuid        null references credentials.source_secrets (id),
    sync_state      jsonb       not null default '{}' check (jsonb_typeof(sync_state) = 'object'),
    last_started_at timestamptz null,
    last_sync_at    timestamptz null,
    last_error_kind text        null check (last_error_kind in ('auth', 'rate_limit', 'provider', 'network', 'store')),
    last_error      text        null check (length(last_error) <= 200),
    failure_count   integer     not null default 0 check (failure_count >= 0),
    next_sync_at    timestamptz not null,
    paused_at       timestamptz null,
    run_generation  bigint      not null default 0,
    run_until       timestamptz null,
    created_at      timestamptz not null,
    constraint accounts_instance unique (source, instance_key)
);

create table sources.sync_runs (
    id         uuid        primary key,
    account_id uuid        not null references sources.accounts (id) on delete cascade,
    trigger    text        not null check (trigger in ('connect', 'schedule', 'manual')),
    started_at timestamptz not null,
    ended_at   timestamptz null,
    outcome    text        null check (outcome in ('ok', 'failed', 'stopped')),
    inserted   integer     not null default 0 check (inserted >= 0),
    changed    integer     not null default 0 check (changed >= 0),
    unchanged  integer     not null default 0 check (unchanged >= 0),
    removed    integer     not null default 0 check (removed >= 0),
    problems   jsonb       not null default '[]' check (jsonb_typeof(problems) = 'array'),
    error      text        null check (length(error) <= 200)
);

create index sync_runs_newest on sources.sync_runs (account_id, started_at desc);
