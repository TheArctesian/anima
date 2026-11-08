-- Day partitions of notifications.deliveries (K2 "Partitions", DB-004, DB-005, D-25).
--
-- Only the table owner may create or drop a partition, and the hub role owns nothing, so the
-- runtime role does both through these SECURITY DEFINER functions, owned by the migration
-- role. search_path lists pg_temp last and every name below is schema-qualified, so a caller's
-- temporary schema cannot shadow a type or table (see events.ensure_day_partition, 0500).
-- There is no default partition: a missing partition is an error.

-- Creates notifications.deliveries_YYYYMMDD for the UTC day that holds `instant`, if it is
-- missing. Safe under concurrent calls. It refuses an instant outside
-- [now() - 31 days, now() + 7 days], so a caller cannot fill the schema with partitions for
-- arbitrary days.
create function notifications.ensure_day_partition(instant pg_catalog.timestamptz)
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
        raise exception 'notifications.ensure_day_partition: % is outside the partition window',
            instant
            using errcode = 'invalid_parameter_value';
    end if;
    utc_day := (instant at time zone 'UTC')::pg_catalog.date;
    child := 'deliveries_' || pg_catalog.to_char(utc_day, 'YYYYMMDD');
    -- Return before any CREATE when the partition exists: the CREATE would lock the parent
    -- even though it then fails as a duplicate.
    if pg_catalog.to_regclass(pg_catalog.format('notifications.%I', child)) is not null then
        return;
    end if;
    begin
        -- Bounds as ISO text with an explicit offset, independent of DateStyle and TimeZone.
        execute pg_catalog.format(
            'create table notifications.%I partition of notifications.deliveries '
            'for values from (%L) to (%L)',
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

revoke execute on function notifications.ensure_day_partition(timestamptz) from public;
grant execute on function notifications.ensure_day_partition(timestamptz) to anima_runtime;

-- Drops the delivery day partitions for UTC days strictly before `cutoff`, and returns their
-- names. Retention (D-25): it refuses a `cutoff` newer than the UTC day of `instant` minus 7
-- days, the shortest period an operator can set. The database clock also bounds `instant`, so
-- a caller cannot claim a later "now" to drop deliveries early.
create function notifications.drop_day_partitions_before(
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
    latest := (
        (least(instant, pg_catalog.now()) at time zone 'UTC') - '7 days'::pg_catalog.interval
    )::pg_catalog.date;
    if cutoff is null or not (cutoff <= latest) then
        raise exception
            'notifications.drop_day_partitions_before: % is inside the delivery retention',
            cutoff
            using errcode = 'insufficient_privilege';
    end if;
    for child in
        select c.relname::pg_catalog.text
        from pg_catalog.pg_inherits i
        join pg_catalog.pg_class c on c.oid = i.inhrelid
        where i.inhparent = 'notifications.deliveries'::pg_catalog.regclass
            and c.relnamespace = 'notifications'::pg_catalog.regnamespace
            and c.relname operator(pg_catalog.~) '^deliveries_[0-9]{8}$'
            and pg_catalog.to_date(pg_catalog.right(c.relname, 8), 'YYYYMMDD') < cutoff
        order by c.relname
    loop
        execute pg_catalog.format('drop table notifications.%I', child);
        return next child;
    end loop;
end;
$$;

revoke execute on function notifications.drop_day_partitions_before(date, timestamptz)
    from public;
grant execute on function notifications.drop_day_partitions_before(date, timestamptz)
    to anima_runtime;
