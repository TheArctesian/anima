-- The runtime group and its default privileges (K2 "Roles", R-016, LOG-006).
--
-- Migrations run as the owner role, so the owner owns every schema, table, sequence and
-- function. The hub serves as a login that is a member of `anima_runtime` and owns nothing.
-- This migration runs first, so the default privileges below reach every object the later
-- migrations create (devices, credentials, events, ...). Exceptions are explicit and live in
-- the owning domain's migration (for example 0500 takes UPDATE and DELETE off the logs).

-- Roles are cluster-wide, so another database (or a concurrent migration) may have created
-- the group already. It must stay a group (NOLOGIN): nobody signs in with its rights alone.
do $$
begin
    if exists (
        select from pg_catalog.pg_roles where rolname = 'anima_runtime' and rolcanlogin
    ) then
        raise exception 'anima_runtime exists and can log in; it must be a NOLOGIN role';
    end if;
    if not exists (select from pg_catalog.pg_roles where rolname = 'anima_runtime') then
        begin
            create role anima_runtime nologin;
        exception
            when duplicate_object or unique_violation then
                null;
        end;
    end if;
end;
$$;

-- For objects the current (owner) role creates from now on, in this database.
alter default privileges grant usage on schemas to anima_runtime;
alter default privileges grant select, insert, update, delete on tables to anima_runtime;
alter default privileges grant usage, select on sequences to anima_runtime;
alter default privileges grant execute on functions to anima_runtime;
