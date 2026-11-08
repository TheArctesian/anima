-- The heartbeat floor follows the settable period (D-25, CR-6): the database refuses to drop a
-- heartbeat partition less than 1 day old, where 0300 said 30 days. The hub passes the operator's
-- period (1 to 400 days, default 30; WP-46). Replaces the body only: the owner, SECURITY DEFINER,
-- search_path and grants are those of 0300 (restated below).

create or replace function checks.drop_day_partitions_before(
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
        (least(instant, pg_catalog.now()) at time zone 'UTC') - '1 day'::pg_catalog.interval
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
            and c.relnamespace = 'checks'::pg_catalog.regnamespace
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
