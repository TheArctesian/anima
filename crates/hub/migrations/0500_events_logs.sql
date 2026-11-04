-- Event log and audit log (K2, LOG-004, LOG-005), partitioned by UTC day (DB-004, D-4).
-- No default partition: events.ensure_day_partition creates the day's partition before each
-- insert, and a missing partition is an error rather than a silent catch-all.

create schema events;

create table events.event_log (
    id uuid not null,
    at timestamptz not null,
    severity text not null check (severity in ('info', 'warning', 'error')),
    source text not null check (source in ('check', 'session', 'device', 'access', 'hub')),
    device_id uuid null,
    device_name text null,
    message text not null,
    primary key (at, id),
    check ((device_id is null) = (device_name is null))
) partition by range (at);

create index event_log_device_at on events.event_log (device_id, at desc)
    where device_id is not null;

create table events.audit_log (
    id uuid not null,
    at timestamptz not null,
    operator_id uuid not null,
    operator_name text not null,
    action text not null,
    target_type text not null,
    target_id text not null,
    client_addr inet not null,
    change jsonb not null,
    primary key (at, id)
) partition by range (at);

create index audit_log_target_at on events.audit_log (target_type, target_id, at desc);

-- Creates events.<parent>_YYYYMMDD for the UTC day that holds `instant`, if it is missing.
-- Safe under concurrent calls: a racing creator makes this call a no-op.
--
-- SECURITY DEFINER: only the table owner may create a partition, and the hub role must not
-- own the tables, because an owner can always UPDATE, DELETE, TRUNCATE or DROP the audit log
-- (LOG-006). The function runs as its owner (the migration role), so partitions it creates
-- are owned by that role too.
--
-- The caller controls its own temporary schema. Unless pg_temp is listed, PostgreSQL searches
-- it FIRST for type and table names, so a caller's `pg_temp.date` domain with a CHECK would run
-- the caller's code as the owner. pg_temp is therefore listed last, and every type and table
-- in the body is schema-qualified anyway. (Functions and operators are never looked up in
-- pg_temp.)
create function events.ensure_day_partition(parent pg_catalog.text, instant pg_catalog.timestamptz)
returns void
language plpgsql
security definer
set search_path = pg_catalog, pg_temp
as $$
declare
    utc_day pg_catalog.date := (instant at time zone 'UTC')::pg_catalog.date;
    child pg_catalog.text := parent || '_' || pg_catalog.to_char(utc_day, 'YYYYMMDD');
begin
    if parent not in ('event_log', 'audit_log') then
        raise exception 'events.ensure_day_partition: unknown table %', parent
            using errcode = 'invalid_parameter_value';
    end if;
    -- Return before any CREATE when the partition exists: the CREATE would lock the parent
    -- even though it then fails as a duplicate (db_004_writes_in_opposite_orders_succeed_on_a_
    -- day_created_ahead deadlocks without this check).
    if pg_catalog.to_regclass(pg_catalog.format('events.%I', child)) is not null then
        return;
    end if;
    begin
        -- Bounds as ISO text with an explicit offset, independent of DateStyle and TimeZone.
        execute pg_catalog.format(
            'create table events.%I partition of events.%I for values from (%L) to (%L)',
            child,
            parent,
            pg_catalog.to_char(utc_day, 'YYYY-MM-DD') || ' 00:00:00+00',
            pg_catalog.to_char(utc_day + 1, 'YYYY-MM-DD') || ' 00:00:00+00'
        );
    exception
        when duplicate_table or unique_violation then
            null;
    end;
end;
$$;

revoke execute on function events.ensure_day_partition(text, timestamptz) from public;

-- The group role the hub login role belongs to (LOG-006, K2): it may read and append, never
-- change or remove. Roles are cluster-wide, so another database (or a concurrent migration)
-- may have created it already. It must stay a group role (NOLOGIN): the hub logs in as its
-- own role, which is a member, so nobody can sign in with the writer rights alone.
do $$
begin
    if exists (
        select from pg_catalog.pg_roles where rolname = 'anima_events_writer' and rolcanlogin
    ) then
        raise exception 'anima_events_writer exists and can log in; it must be a NOLOGIN role';
    end if;
    if not exists (select from pg_catalog.pg_roles where rolname = 'anima_events_writer') then
        begin
            create role anima_events_writer nologin;
        exception
            when duplicate_object or unique_violation then
                null;
        end;
    end if;
end;
$$;

grant usage on schema events to anima_events_writer;
grant select, insert on events.event_log, events.audit_log to anima_events_writer;
grant execute on function events.ensure_day_partition(text, timestamptz) to anima_events_writer;
