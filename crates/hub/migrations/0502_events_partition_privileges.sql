-- New day partitions get their privileges from this function alone, not from default
-- privileges (K2 "Roles", R-016, R-023, LOG-006).
--
-- Migration 0500's events.ensure_day_partition revoked UPDATE, DELETE and TRUNCATE from
-- anima_runtime on each new partition and kept every other grant the default privileges
-- added. A default that grants to another role or to PUBLIC (for example
-- `alter default privileges grant update, delete on tables to some_group`) therefore reached
-- every new partition. This replacement revokes every privilege from every grantee except the
-- owner, including PUBLIC, and then grants anima_runtime exactly SELECT and INSERT.
-- The DO block at the end applies the same reset once to the partitions that already exist.
--
-- The signature, SECURITY DEFINER, search_path and schema-qualified names are those of 0500
-- (see its comments for why pg_temp is listed last).
create or replace function events.ensure_day_partition(
    parent pg_catalog.text,
    instant pg_catalog.timestamptz
)
returns void
language plpgsql
security definer
set search_path = pg_catalog, pg_temp
as $$
declare
    utc_day pg_catalog.date := (instant at time zone 'UTC')::pg_catalog.date;
    child pg_catalog.text := parent || '_' || pg_catalog.to_char(utc_day, 'YYYYMMDD');
    holder pg_catalog.oid;
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
        -- Every grant the default privileges just added, to any role, goes.
        execute pg_catalog.format('revoke all on events.%I from public cascade', child);
        for holder in
            select distinct a.grantee
            from pg_catalog.pg_class c
            cross join lateral pg_catalog.aclexplode(c.relacl) a
            where c.oid = pg_catalog.to_regclass(pg_catalog.format('events.%I', child))
                and a.grantee operator(pg_catalog.<>) c.relowner
                and a.grantee operator(pg_catalog.<>) 0
        loop
            execute pg_catalog.format(
                'revoke all on events.%I from %I cascade',
                child,
                pg_catalog.pg_get_userbyid(holder)
            );
        end loop;
        execute pg_catalog.format(
            'grant select, insert on events.%I to anima_runtime',
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

-- Once, for the partitions that already exist (an installation upgraded from 0500): the same
-- reset, so a grant that 0500's function left on an older partition does not reach the hub or
-- trip its start check. CASCADE (here and in the function) also removes grants that a grantee
-- passed on with GRANT OPTION, which would otherwise abort the revoke.
do $$
declare
    existing pg_catalog.regclass;
    holder pg_catalog.oid;
begin
    for existing in
        select i.inhrelid::pg_catalog.regclass
        from pg_catalog.pg_inherits i
        where i.inhparent in (
            pg_catalog.to_regclass('events.event_log'),
            pg_catalog.to_regclass('events.audit_log')
        )
    loop
        execute pg_catalog.format('revoke all on %s from public cascade', existing);
        for holder in
            select distinct a.grantee
            from pg_catalog.pg_class c
            cross join lateral pg_catalog.aclexplode(c.relacl) a
            where c.oid = existing
                and a.grantee operator(pg_catalog.<>) c.relowner
                and a.grantee operator(pg_catalog.<>) 0
        loop
            execute pg_catalog.format(
                'revoke all on %s from %I cascade',
                existing,
                pg_catalog.pg_get_userbyid(holder)
            );
        end loop;
        execute pg_catalog.format('grant select, insert on %s to anima_runtime', existing);
    end loop;
end;
$$;
