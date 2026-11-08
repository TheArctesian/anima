-- The settings a new check starts with (K2, D-26, DB-011).
--
-- One row, so `id bool primary key default true check (id)` allows no second one. Each column
-- has the default and the bounds of the matching column of checks.checks (migration 0300), and
-- the constraints carry the same names, so a refused write names the same field. The row is
-- seeded here with the schema defaults. The hub's default check reads it; createCheck keeps
-- its own input defaults.
--
-- Privileges: migration 0001's default privileges give `anima_runtime` SELECT, INSERT, UPDATE
-- and DELETE. The row is never inserted or deleted after this migration, so those two go.

create table checks.defaults (
    id boolean not null default true,
    interval_seconds integer not null default 60,
    retries integer not null default 3,
    retry_interval_seconds integer not null default 20,
    degraded_threshold_ms integer not null default 1000,
    constraint defaults_pkey primary key (id),
    constraint defaults_id_check check (id),
    constraint checks_interval_seconds_check check (interval_seconds between 10 and 86400),
    constraint checks_retries_check check (retries between 0 and 10),
    constraint checks_retry_interval_seconds_check
        check (retry_interval_seconds between 5 and 3600),
    constraint checks_degraded_threshold_ms_check
        check (degraded_threshold_ms between 1 and 600000)
);

insert into checks.defaults default values;

revoke insert, delete on checks.defaults from anima_runtime;
