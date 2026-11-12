-- Session recordings (K2 range 0660-0679; RA-040, SEC-003, D-13, D-14, K4 §11).
--
-- One row per recorded session. The row is written before the first part of the file, and it
-- holds the only copy of the recording's key, sealed with the vault (context
-- `sessions.recordings:<id>`): deleting the row makes the file unreadable (D-13). Like
-- `sessions.sessions`, the row outlives the device: `device_id` has no foreign key and
-- `device_name` is a copy.
--
-- `session_id` refuses the delete of its session (`on delete restrict`): session rows are
-- kept (RA-008, migration 0600 revokes their delete from the runtime role), and a recording
-- never loses the session it belongs to.
--
-- Privileges (R-016, K2 "Roles"): anima_runtime gets SELECT, INSERT, UPDATE and DELETE on this
-- table from the default privileges that migration 0001 sets for the owner role; the end of
-- this file narrows UPDATE.

create table sessions.recordings (
    id uuid primary key,
    session_id uuid not null unique,
    device_id uuid null,
    device_name text not null,
    protocol text not null,
    format text not null,
    key_sealed bytea not null,
    started_at timestamptz not null,
    ended_at timestamptz null,
    size_bytes bigint not null default 0,
    complete boolean not null default false,
    stopped_reason text null,
    expires_at timestamptz not null,
    constraint recordings_session_id_fkey foreign key (session_id)
        references sessions.sessions (id) on delete restrict,
    constraint recordings_format_check check (format in ('guacamole', 'asciicast')),
    constraint recordings_stopped_reason_check check (
        stopped_reason in ('size_limit', 'disk_full', 'write_failed')
    )
);

-- The daily retention job deletes rows whose `expires_at` has passed (D-25).
create index recordings_expires_at_idx on sessions.recordings (expires_at);

-- A recording's key is sealed once, for its own file, and never replaced: a new key over an
-- existing file would reuse its nonces (K4 §11). The runtime role may set only the end of a
-- recording, and the retention job may move its expiry; every other column stays as inserted.
revoke update on sessions.recordings from anima_runtime;
grant update (ended_at, size_bytes, complete, stopped_reason, expires_at)
    on sessions.recordings to anima_runtime;
