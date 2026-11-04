-- Event log and audit log (K2, LOG-004, LOG-005), partitioned by UTC day (DB-004, D-4).
-- No default partition: events.ensure_day_partition creates the day's partition before each
-- insert, and a missing partition is an error rather than a silent catch-all.
--
-- Privileges (K2 "Roles", R-016, LOG-006): migration 0001's default privileges give
-- `anima_runtime` SELECT, INSERT, UPDATE and DELETE on every table created here, including
-- every partition the functions below create (they run as the owner). The runtime role may
-- only read and append, so UPDATE, DELETE and TRUNCATE are revoked from the parents here and
-- from each partition when it is created. Only the owner can create or drop a partition; the
-- runtime role does both through the SECURITY DEFINER functions below.

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

revoke update, delete, truncate on events.event_log, events.audit_log from anima_runtime;

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
    if parent collate pg_catalog."C" not in ('event_log', 'audit_log') then
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
        -- The default privileges just granted UPDATE and DELETE on the new partition.
        execute pg_catalog.format(
            'revoke update, delete, truncate on events.%I from anima_runtime',
            child
        );
    exception
        when duplicate_table or unique_violation then
            null;
    end;
end;
$$;

revoke execute on function events.ensure_day_partition(text, timestamptz) from public;
grant execute on function events.ensure_day_partition(text, timestamptz) to anima_runtime;

-- Drops the day partitions of `parent` for UTC days strictly before `cutoff`, and returns
-- their names. Retention (DB-005, LOG-006): it refuses a `cutoff` newer than the UTC day of
-- `instant` minus 90 days (event_log) or 1 year (audit_log). The caller passes its clock as
-- `instant`, but the database clock also bounds it, so a caller cannot claim a later "now"
-- to drop entries early. SECURITY DEFINER and search_path as for ensure_day_partition.
create function events.drop_day_partitions_before(
    parent pg_catalog.text,
    cutoff pg_catalog.date,
    instant pg_catalog.timestamptz
)
returns setof pg_catalog.text
language plpgsql
security definer
set search_path = pg_catalog, pg_temp
as $$
declare
    retention pg_catalog.interval := case parent collate pg_catalog."C"
        when 'event_log' then '90 days'::pg_catalog.interval
        when 'audit_log' then '1 year'::pg_catalog.interval
    end;
    latest pg_catalog.date;
    child pg_catalog.text;
begin
    if retention is null then
        raise exception 'events.drop_day_partitions_before: unknown table %', parent
            using errcode = 'invalid_parameter_value';
    end if;
    -- Timestamp (not timestamptz) arithmetic, so the session TimeZone cannot move the bound.
    latest := (
        (least(instant, pg_catalog.now()) at time zone 'UTC') - retention
    )::pg_catalog.date;
    if cutoff is null or cutoff > latest then
        raise exception 'events.drop_day_partitions_before: % is inside the retention of %',
            cutoff, parent
            using errcode = 'insufficient_privilege';
    end if;
    for child in
        select c.relname::pg_catalog.text
        from pg_catalog.pg_inherits i
        join pg_catalog.pg_class c on c.oid = i.inhrelid
        where i.inhparent = pg_catalog.to_regclass(pg_catalog.format('events.%I', parent))
            and c.relname operator(pg_catalog.~) ('^' || parent || '_[0-9]{8}$')
            and pg_catalog.to_date(pg_catalog.right(c.relname, 8), 'YYYYMMDD') < cutoff
        order by c.relname
    loop
        execute pg_catalog.format('drop table events.%I', child);
        return next child;
    end loop;
end;
$$;

revoke execute on function events.drop_day_partitions_before(text, date, timestamptz) from public;
grant execute on function events.drop_day_partitions_before(text, date, timestamptz) to anima_runtime;
