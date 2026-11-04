-- Checks, their heartbeats and the uptime aggregates (K2, CHK-009, DB-004, DB-008, DB-011).
--
-- Every default lives here, not only in the code (DB-011). A check row also holds the state the
-- scheduler keeps for it (status, failures, due time, last beat), so reading a device's status
-- reads one row per check instead of raw heartbeats (CHK-011).
--
-- Heartbeats are partitioned by UTC day (DB-004) with no default partition:
-- checks.ensure_day_partition creates the day's partition before each insert, and a missing
-- partition is an error rather than a silent catch-all. Heartbeats have no foreign key, so
-- deleting a device or check leaves them behind; the retention job deletes them, and drops
-- whole partitions after 30 days (DB-005, DB-009).
--
-- Privileges (K2 "Roles", R-016): migration 0001's default privileges give `anima_runtime`
-- SELECT, INSERT, UPDATE and DELETE on every table created here, including every partition the
-- functions below create (they run as the owner). Only the owner can create or drop a
-- partition; the runtime role does both through the SECURITY DEFINER functions below.

create schema checks;

create table checks.checks (
    id uuid not null,
    device_id uuid not null,
    parent_id uuid null,
    type text not null,
    name text not null,
    interval_seconds integer not null default 60,
    retries integer not null default 3,
    retry_interval_seconds integer not null default 20,
    degraded_threshold_ms integer not null default 1000,
    invert boolean not null default false,
    paused boolean not null default false,
    config jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    status text not null default 'unknown',
    failures integer not null default 0,
    next_due_at timestamptz null,
    last_beat_at timestamptz null,
    cert_not_after timestamptz null,
    resolved_ip inet null,
    push_token text null,
    secrets_sealed bytea null,
    constraint checks_pkey primary key (id),
    -- Target of the parent key below: a parent is a check on the same device.
    constraint checks_device_id_id_key unique (device_id, id),
    constraint checks_device_id_name_key unique (device_id, name),
    constraint checks_push_token_key unique (push_token),
    constraint checks_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    -- MATCH SIMPLE: a null parent_id skips the check. Deleting the parent nulls only
    -- parent_id, so the child keeps its device.
    constraint checks_parent_fkey foreign key (device_id, parent_id)
        references checks.checks (device_id, id) on delete set null (parent_id),
    constraint checks_parent_not_self_check check (parent_id <> id),
    -- Fixed identifiers (CHK-009): never change the meaning of one; add a new one instead.
    constraint checks_type_check
        check (type in ('icmp', 'tcp', 'tls', 'http', 'dns', 'push', 'group')),
    constraint checks_name_check check (char_length(name) between 1 and 200),
    constraint checks_interval_seconds_check check (interval_seconds between 10 and 86400),
    constraint checks_retries_check check (retries between 0 and 10),
    constraint checks_retry_interval_seconds_check
        check (retry_interval_seconds between 5 and 3600),
    constraint checks_degraded_threshold_ms_check
        check (degraded_threshold_ms between 1 and 600000),
    constraint checks_config_check check (jsonb_typeof(config) = 'object'),
    constraint checks_status_check
        check (status in ('up', 'down', 'degraded', 'maintenance', 'unknown')),
    constraint checks_failures_check check (failures >= 0),
    constraint checks_push_token_check check ((push_token is not null) = (type = 'push'))
);

create index checks_device_id_idx on checks.checks (device_id);
create index checks_next_due_at_idx on checks.checks (next_due_at) where not paused;

create table checks.heartbeats (
    check_id uuid not null,
    at timestamptz not null,
    status text not null,
    response_time_ms integer null,
    message text null,
    constraint heartbeats_pkey primary key (check_id, at),
    constraint heartbeats_status_check
        check (status in ('up', 'down', 'degraded', 'maintenance', 'unknown')),
    constraint heartbeats_response_time_ms_check check (response_time_ms >= 0),
    constraint heartbeats_message_check check (char_length(message) <= 512)
) partition by range (at);

-- One row per check and UTC minute, hour or day (DB-008), upserted with each heartbeat.
create table checks.uptime_minute (
    check_id uuid not null,
    bucket timestamptz not null,
    up_count integer not null default 0,
    down_count integer not null default 0,
    degraded_count integer not null default 0,
    maintenance_count integer not null default 0,
    unknown_count integer not null default 0,
    response_count integer not null default 0,
    avg_response_ms real null,
    constraint uptime_minute_pkey primary key (check_id, bucket),
    constraint uptime_minute_check_id_fkey foreign key (check_id)
        references checks.checks (id) on delete cascade,
    constraint uptime_minute_counts_check check (
        up_count >= 0 and down_count >= 0 and degraded_count >= 0
        and maintenance_count >= 0 and unknown_count >= 0 and response_count >= 0
    ),
    constraint uptime_minute_avg_response_ms_check check (avg_response_ms >= 0)
);

create table checks.uptime_hour (
    check_id uuid not null,
    bucket timestamptz not null,
    up_count integer not null default 0,
    down_count integer not null default 0,
    degraded_count integer not null default 0,
    maintenance_count integer not null default 0,
    unknown_count integer not null default 0,
    response_count integer not null default 0,
    avg_response_ms real null,
    constraint uptime_hour_pkey primary key (check_id, bucket),
    constraint uptime_hour_check_id_fkey foreign key (check_id)
        references checks.checks (id) on delete cascade,
    constraint uptime_hour_counts_check check (
        up_count >= 0 and down_count >= 0 and degraded_count >= 0
        and maintenance_count >= 0 and unknown_count >= 0 and response_count >= 0
    ),
    constraint uptime_hour_avg_response_ms_check check (avg_response_ms >= 0)
);

create table checks.uptime_day (
    check_id uuid not null,
    bucket timestamptz not null,
    up_count integer not null default 0,
    down_count integer not null default 0,
    degraded_count integer not null default 0,
    maintenance_count integer not null default 0,
    unknown_count integer not null default 0,
    response_count integer not null default 0,
    avg_response_ms real null,
    constraint uptime_day_pkey primary key (check_id, bucket),
    constraint uptime_day_check_id_fkey foreign key (check_id)
        references checks.checks (id) on delete cascade,
    constraint uptime_day_counts_check check (
        up_count >= 0 and down_count >= 0 and degraded_count >= 0
        and maintenance_count >= 0 and unknown_count >= 0 and response_count >= 0
    ),
    constraint uptime_day_avg_response_ms_check check (avg_response_ms >= 0)
);

-- Creates checks.heartbeats_YYYYMMDD for the UTC day that holds `instant`, if it is missing.
-- Safe under concurrent calls: a racing creator makes this call a no-op.
--
-- SECURITY DEFINER: only the table owner may create a partition, and the hub role owns
-- nothing. The function runs as its owner (the migration role), so partitions it creates are
-- owned by that role too.
--
-- The caller controls its own temporary schema. Unless pg_temp is listed, PostgreSQL searches
-- it FIRST for type and table names, so a caller's `pg_temp.date` domain with a CHECK would run
-- the caller's code as the owner. pg_temp is therefore listed last, and every type and table
-- in the body is schema-qualified anyway. (Functions and operators are never looked up in
-- pg_temp.)
--
-- Only finite days in years 1 to 9999 have a partition: other years do not fit the eight
-- digits of the name, so the drop function could never find them.
create function checks.ensure_day_partition(instant pg_catalog.timestamptz)
returns void
language plpgsql
security definer
set search_path = pg_catalog, pg_temp
as $$
declare
    utc_day pg_catalog.date;
    child pg_catalog.text;
begin
    if instant is null or not pg_catalog.isfinite(instant) then
        raise exception 'checks.ensure_day_partition: no day for %', instant
            using errcode = 'invalid_parameter_value';
    end if;
    utc_day := (instant at time zone 'UTC')::pg_catalog.date;
    if utc_day < '0001-01-01'::pg_catalog.date or utc_day > '9999-12-31'::pg_catalog.date then
        raise exception 'checks.ensure_day_partition: no day for %', instant
            using errcode = 'invalid_parameter_value';
    end if;
    child := 'heartbeats_' || pg_catalog.to_char(utc_day, 'YYYYMMDD');
    -- Return before any CREATE when the partition exists: the CREATE would lock the parent
    -- even though it then fails as a duplicate, and so block or deadlock open writers.
    if pg_catalog.to_regclass(pg_catalog.format('checks.%I', child)) is not null then
        return;
    end if;
    begin
        -- Bounds as ISO text with an explicit offset, independent of DateStyle and TimeZone.
        execute pg_catalog.format(
            'create table checks.%I partition of checks.heartbeats for values from (%L) to (%L)',
            child,
            pg_catalog.to_char(utc_day, 'YYYY-MM-DD') || ' 00:00:00+00',
            pg_catalog.to_char(utc_day + 1, 'YYYY-MM-DD') || ' 00:00:00+00'
        );
    exception
        when duplicate_table or unique_violation then
            null;
    end;
end;
$$;

revoke execute on function checks.ensure_day_partition(timestamptz) from public;
grant execute on function checks.ensure_day_partition(timestamptz) to anima_runtime;

-- Drops the heartbeat day partitions for UTC days strictly before `cutoff`, and returns their
-- names. Retention (DB-005, DB-009): it refuses a `cutoff` newer than the UTC day of `instant`
-- minus 30 days. The caller passes its clock as `instant`, but the database clock also bounds
-- it, so a caller cannot claim a later "now" to drop heartbeats early. SECURITY DEFINER and
-- search_path as for ensure_day_partition.
create function checks.drop_day_partitions_before(
    cutoff pg_catalog.date,
    instant pg_catalog.timestamptz
)
returns setof pg_catalog.text
language plpgsql
security definer
set search_path = pg_catalog, pg_temp
as $$
declare
    latest pg_catalog.date;
    child pg_catalog.text;
begin
    -- Timestamp (not timestamptz) arithmetic, so the session TimeZone cannot move the bound.
    latest := (
        (least(instant, pg_catalog.now()) at time zone 'UTC') - '30 days'::pg_catalog.interval
    )::pg_catalog.date;
    if cutoff is null or not (cutoff <= latest) then
        raise exception 'checks.drop_day_partitions_before: % is inside the heartbeat retention',
            cutoff
            using errcode = 'insufficient_privilege';
    end if;
    for child in
        select c.relname::pg_catalog.text
        from pg_catalog.pg_inherits i
        join pg_catalog.pg_class c on c.oid = i.inhrelid
        where i.inhparent = 'checks.heartbeats'::pg_catalog.regclass
            and c.relname operator(pg_catalog.~) '^heartbeats_[0-9]{8}$'
            and pg_catalog.to_date(pg_catalog.right(c.relname, 8), 'YYYYMMDD') < cutoff
        order by c.relname
    loop
        execute pg_catalog.format('drop table checks.%I', child);
        return next child;
    end loop;
end;
$$;

revoke execute on function checks.drop_day_partitions_before(date, timestamptz) from public;
grant execute on function checks.drop_day_partitions_before(date, timestamptz) to anima_runtime;
