-- The extensions domain's switches (K-E8 range 2020-2029; EXT-001, EXT-003).
--
-- No row means off, and nothing seeds this table: `setExtension` is the only writer. A row
-- whose key is not in the registry is ignored. Privileges: migration 0001's default privileges
-- give `anima_runtime` what it needs on the new schema (K2 "Roles").
create schema extensions;

create table extensions.switches (
    key        text        primary key check (key ~ '^[a-z][a-z_]*$' and length(key) <= 40),
    is_on      boolean     not null,
    changed_at timestamptz not null,
    changed_by text        not null
);
