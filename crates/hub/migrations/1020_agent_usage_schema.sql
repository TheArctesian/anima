-- The use of the AI agent programs on each device: the sessions of Claude Code and Codex with
-- their counts of prompts, tool calls and tokens by UTC day and by model, and what each daemon
-- can read (K2 1020-1039; R-135; AIS-018 to AIS-024; K8 Section 4).
--
-- `states` holds one row per device whose daemon sent a usage state it knows: `reading`,
-- `system_unit` or `turned_off` (AIS-024). `sessions` holds one row per session, keyed by the
-- device, the source and the program's session ID, so a later collector of the same session
-- uses the same row (AIS-023); its start, end and repository are those of the entry stored
-- last, and `model` is the model of its token rows with the most output tokens, the first by
-- name (byte order) on a tie, null when it has none. `session_days` holds the prompts and the
-- tool calls of a session on one UTC day, and `day_models` the tokens of one model on that
-- day. Storing an entry replaces the session's day and its model rows in one transaction.
--
-- No column holds text of a session, a title, the path of a folder, a login, an account or a
-- usage limit (AIS-018, AIS-022). The checks repeat the rules of K8 Section 4. Every count is
-- a whole number from 0 to 2^53 - 1.
--
-- Deleting a device deletes its sessions, their days and model rows, and its state (DEV-015).
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` USAGE on
-- the new schema; SELECT, INSERT, UPDATE and DELETE on the new tables; and USAGE and SELECT on
-- the identity sequence. The migration only creates empty tables, so it is cheap on a large
-- database (DB-010).

create schema agent_usage;

create table agent_usage.states (
    device_id uuid not null,
    state text not null,
    constraint states_pkey primary key (device_id),
    constraint states_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint states_state_check check (state in ('reading', 'system_unit', 'turned_off'))
);

create table agent_usage.sessions (
    id bigint generated always as identity,
    device_id uuid not null,
    source text not null,
    external_id text not null,
    started_at timestamptz not null,
    ended_at timestamptz not null,
    repository text,
    model text,
    constraint sessions_pkey primary key (id),
    constraint sessions_key unique (device_id, source, external_id),
    constraint sessions_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint sessions_source_check check (source in ('claude_code', 'codex')),
    constraint sessions_external_id_check check (external_id ~ '^[0-9A-Za-z_-]{1,64}$'),
    constraint sessions_times_check check (started_at <= ended_at),
    constraint sessions_repository_check check (
        octet_length(repository) <= 200
        and repository ~ '^[a-z0-9.-]+(/[A-Za-z0-9._-]{1,100}){2,8}$'
    ),
    constraint sessions_model_check check (model ~ '^[0-9A-Za-z._:-]{1,64}$')
);

create table agent_usage.session_days (
    session_id bigint not null,
    day date not null,
    prompts bigint not null,
    tool_calls bigint not null,
    constraint session_days_pkey primary key (session_id, day),
    constraint session_days_session_id_fkey foreign key (session_id)
        references agent_usage.sessions (id) on delete cascade,
    constraint session_days_prompts_check check (prompts between 0 and 9007199254740991),
    constraint session_days_tool_calls_check check (tool_calls between 0 and 9007199254740991)
);

-- The query reads a range of days first.
create index session_days_day_idx on agent_usage.session_days (day);

create table agent_usage.day_models (
    session_id bigint not null,
    day date not null,
    model text not null,
    input_tokens bigint not null,
    output_tokens bigint not null,
    cache_read_tokens bigint not null,
    cache_write_tokens bigint not null,
    constraint day_models_pkey primary key (session_id, day, model),
    constraint day_models_session_day_fkey foreign key (session_id, day)
        references agent_usage.session_days (session_id, day) on delete cascade,
    constraint day_models_model_check check (model ~ '^[0-9A-Za-z._:-]{1,64}$'),
    constraint day_models_counts_check check (
        input_tokens between 0 and 9007199254740991
        and output_tokens between 0 and 9007199254740991
        and cache_read_tokens between 0 and 9007199254740991
        and cache_write_tokens between 0 and 9007199254740991
    )
);
