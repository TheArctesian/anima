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
create function events.ensure_day_partition(parent text, instant timestamptz)
returns void
language plpgsql
set search_path = pg_catalog
as $$
declare
    utc_day date := (instant at time zone 'UTC')::date;
    child text := parent || '_' || to_char(utc_day, 'YYYYMMDD');
begin
    if parent not in ('event_log', 'audit_log') then
        raise exception 'events.ensure_day_partition: unknown table %', parent
            using errcode = 'invalid_parameter_value';
    end if;
    if to_regclass(format('events.%I', child)) is not null then
        return;
    end if;
    begin
        execute format(
            'create table events.%I partition of events.%I for values from (%L) to (%L)',
            child,
            parent,
            (utc_day::timestamp at time zone 'UTC'),
            ((utc_day + 1)::timestamp at time zone 'UTC')
        );
    exception
        when duplicate_table or unique_violation then
            null;
    end;
end;
$$;
