-- The event_log floor follows the settable period (D-25, CR-6): the database refuses to drop an
-- event_log partition less than 30 days old, where 0500 said 90 days. audit_log stays 1 year
-- (LOG-006). Replaces the body only: the owner, SECURITY DEFINER, search_path and grants are
-- those of 0500 (restated below).

create or replace function events.drop_day_partitions_before(
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
        when 'event_log' then '30 days'::pg_catalog.interval
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
