-- Re-checks that anima_runtime is a plain group, after every migration that grants to it
-- (K2 "Roles", R-016, LOG-006).
--
-- One PostgreSQL cluster serves one Anima installation (DB-001, R-019). anima_runtime is a
-- cluster-wide role: two installations in one cluster would share it, and each one's grants
-- would reach the other's hub.
--
-- Migration 0001 checks the group's attributes before it creates the group. If another
-- database's migration creates it between that check and 0001's CREATE ROLE, 0001 swallows
-- the duplicate_object error and has not seen the attributes the other creator gave it. This
-- migration checks again, so a group with more rights than a plain NOLOGIN group stops the
-- migration in both cases.
--
-- Migration 0500 grants EXECUTE on its two partition functions to anima_runtime explicitly.
-- That deliberately duplicates 0001's default EXECUTE privilege: the functions are the only
-- route to create or drop a partition, so their grants stay visible beside them and survive a
-- change to the defaults.
do $$
begin
    if exists (
        select from pg_catalog.pg_roles r
        where r.rolname = 'anima_runtime'
            and (
                r.rolcanlogin or r.rolsuper or r.rolcreaterole or r.rolcreatedb or r.rolbypassrls
                or exists (select from pg_catalog.pg_auth_members m where m.member = r.oid)
            )
    ) then
        raise exception 'anima_runtime exists with more rights than a plain NOLOGIN group';
    end if;
    if not exists (select from pg_catalog.pg_roles where rolname = 'anima_runtime') then
        raise exception 'anima_runtime is missing; migration 0001 creates it';
    end if;
end;
$$;
