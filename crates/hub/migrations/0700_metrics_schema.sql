-- Resource metrics: series, samples and their aggregates, and GPUs (K2, HUB-005, DB-004,
-- DB-005, DB-008, DB-009, DB-011, DEV-015).
--
-- Every default lives here, not only in the code (DB-011). One `series` row per device, kind and
-- label (a mount point, an interface name, or empty). Samples are partitioned by UTC day
-- (DB-004) with no default partition: metrics.ensure_day_partition creates the day's partition
-- before each insert, and a missing partition is an error rather than a silent catch-all.
-- Samples have no foreign key, as heartbeats have none: deleting a device cascades to its
-- series and aggregates, the retention job deletes the samples that are left (DEV-015), and
-- drops whole partitions after 30 days (DB-005, DB-009).
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` SELECT,
-- INSERT, UPDATE and DELETE on every table created here, including every partition the
-- functions below create (they run as the owner). Only the owner can create or drop a
-- partition; the runtime role does both through the SECURITY DEFINER functions below. The
-- runtime role keeps DELETE on samples, because the retention job deletes the samples of
-- deleted series row by row, so the drop function's 30-day refusal guards against mistakes,
-- not against a hostile runtime role. Only new tables are created here, so the migration is
-- cheap on a large database (DB-010).

create schema metrics;

create table metrics.series (
    id uuid not null,
    device_id uuid not null,
    kind text not null,
    -- A mount point, an interface name, or '' for kinds that have no label.
    label text not null default '',
    -- The GPU a GPU or VRAM series belongs to; null for the other kinds.
    gpu_index integer null,
    constraint series_pkey primary key (id),
    constraint series_device_id_kind_label_key unique (device_id, kind, label),
    constraint series_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint series_kind_check check (kind in (
        'cpu_percent', 'memory_percent', 'memory_bytes', 'disk_percent', 'disk_bytes',
        'network_in_bps', 'network_out_bps', 'uptime_seconds', 'load_1', 'load_5', 'load_15',
        'gpu_percent', 'vram_percent', 'vram_bytes', 'gpu_temperature_c', 'gpu_power_w'
    )),
    constraint series_label_check check (octet_length(label) <= 4096),
    constraint series_gpu_index_check check (gpu_index >= 0)
);

create table metrics.samples (
    series_id uuid not null,
    at timestamptz not null,
    value double precision not null,
    constraint samples_pkey primary key (series_id, at)
) partition by range (at);

-- One row per series and UTC minute, hour or day (DB-008), upserted with each sample. `avg` is
-- the mean of the `count` samples in the bucket.
create table metrics.minute (
    series_id uuid not null,
    bucket timestamptz not null,
    avg double precision not null,
    min double precision not null,
    max double precision not null,
    count integer not null default 1,
    constraint minute_pkey primary key (series_id, bucket),
    constraint minute_series_id_fkey foreign key (series_id)
        references metrics.series (id) on delete cascade,
    constraint minute_count_check check (count >= 1)
);

create table metrics.hour (
    series_id uuid not null,
    bucket timestamptz not null,
    avg double precision not null,
    min double precision not null,
    max double precision not null,
    count integer not null default 1,
    constraint hour_pkey primary key (series_id, bucket),
    constraint hour_series_id_fkey foreign key (series_id)
        references metrics.series (id) on delete cascade,
    constraint hour_count_check check (count >= 1)
);

create table metrics.day (
    series_id uuid not null,
    bucket timestamptz not null,
    avg double precision not null,
    min double precision not null,
    max double precision not null,
    count integer not null default 1,
    constraint day_pkey primary key (series_id, bucket),
    constraint day_series_id_fkey foreign key (series_id)
        references metrics.series (id) on delete cascade,
    constraint day_count_check check (count >= 1)
);

-- The GPUs found on a device (MET-006). Nothing writes them yet; the table is here so the
-- discovery work needs no later migration.
create table metrics.gpus (
    device_id uuid not null,
    index integer not null,
    name text not null,
    vendor text not null,
    has_vram boolean not null default true,
    found_at timestamptz not null default now(),
    constraint gpus_pkey primary key (device_id, index),
    constraint gpus_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint gpus_index_check check (index >= 0),
    constraint gpus_name_check check (char_length(name) <= 512),
    constraint gpus_vendor_check check (vendor in ('nvidia', 'amd', 'intel', 'apple'))
);

-- When the hub last looked for GPUs on a device, and the boot time it saw then (MET-006).
create table metrics.discovery (
    device_id uuid not null,
    found_at timestamptz not null default now(),
    boot_at timestamptz null,
    constraint discovery_pkey primary key (device_id),
    constraint discovery_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade
);

-- Creates metrics.samples_YYYYMMDD for the UTC day that holds `instant`, if it is missing.
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
-- It refuses an instant outside [now() - 31 days, now() + 7 days], so a caller cannot fill the
-- schema with partitions for arbitrary days (samples are written at collection time, and the
-- scheduler creates at most a few days ahead).
create function metrics.ensure_day_partition(instant pg_catalog.timestamptz)
returns void
language plpgsql
security definer
set search_path = pg_catalog, pg_temp
as $$
declare
    utc_day pg_catalog.date;
    child pg_catalog.text;
begin
    -- Timestamp (not timestamptz) arithmetic, so the session TimeZone cannot move the bounds.
    if instant is null
        or (instant at time zone 'UTC')
            < (pg_catalog.now() at time zone 'UTC') - '31 days'::pg_catalog.interval
        or (instant at time zone 'UTC')
            > (pg_catalog.now() at time zone 'UTC') + '7 days'::pg_catalog.interval
    then
        raise exception 'metrics.ensure_day_partition: % is outside the partition window',
            instant
            using errcode = 'invalid_parameter_value';
    end if;
    utc_day := (instant at time zone 'UTC')::pg_catalog.date;
    child := 'samples_' || pg_catalog.to_char(utc_day, 'YYYYMMDD');
    -- Return before any CREATE when the partition exists: the CREATE would lock the parent
    -- even though it then fails as a duplicate, and so block or deadlock open writers.
    if pg_catalog.to_regclass(pg_catalog.format('metrics.%I', child)) is not null then
        return;
    end if;
    begin
        -- Bounds as ISO text with an explicit offset, independent of DateStyle and TimeZone.
        execute pg_catalog.format(
            'create table metrics.%I partition of metrics.samples for values from (%L) to (%L)',
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

revoke execute on function metrics.ensure_day_partition(timestamptz) from public;
grant execute on function metrics.ensure_day_partition(timestamptz) to anima_runtime;

-- Drops the sample day partitions for UTC days strictly before `cutoff`, and returns their
-- names. Retention (DB-005, DB-009): it refuses a `cutoff` newer than the UTC day of `instant`
-- minus 30 days. The caller passes its clock as `instant`, but the database clock also bounds
-- it, so a caller cannot claim a later "now" to drop samples early. SECURITY DEFINER and
-- search_path as for ensure_day_partition.
create function metrics.drop_day_partitions_before(
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
        raise exception 'metrics.drop_day_partitions_before: % is inside the sample retention',
            cutoff
            using errcode = 'insufficient_privilege';
    end if;
    for child in
        select c.relname::pg_catalog.text
        from pg_catalog.pg_inherits i
        join pg_catalog.pg_class c on c.oid = i.inhrelid
        where i.inhparent = 'metrics.samples'::pg_catalog.regclass
            and c.relnamespace = 'metrics'::pg_catalog.regnamespace
            and c.relname operator(pg_catalog.~) '^samples_[0-9]{8}$'
            and pg_catalog.to_date(pg_catalog.right(c.relname, 8), 'YYYYMMDD') < cutoff
        order by c.relname
    loop
        execute pg_catalog.format('drop table metrics.%I', child);
        return next child;
    end loop;
end;
$$;

revoke execute on function metrics.drop_day_partitions_before(date, timestamptz) from public;
grant execute on function metrics.drop_day_partitions_before(date, timestamptz) to anima_runtime;
