-- The hash chain of the audit log (LOG-021 to LOG-028, R-136): one link for each audit entry.
--
-- The links have their own table, so this migration changes no audit entry (LOG-006, LOG-026);
-- the hub links the entries that exist at its first start. The table has no partitions: a link
-- holds no value of its entry, only its position, its ID and time and two SHA-256 hashes
-- (LOG-007), so it stays when the retention drops the entry and the chain stays whole from
-- link 1 (LOG-025).
--
-- Privileges (K2 "Roles", R-016): the runtime role may only read and append. As 0502 does for a
-- new partition, every grant that a default privilege added, to any role and to PUBLIC, goes,
-- and anima_runtime gets exactly SELECT and INSERT. The start check counts this table among the
-- logs.

create table events.audit_chain (
    seq bigint primary key check (seq >= 1),
    entry_id uuid not null unique,
    entry_at timestamptz not null,
    previous bytea not null check (octet_length(previous) = 32),
    hash bytea not null check (octet_length(hash) = 32)
);

do $$
declare
    holder pg_catalog.oid;
begin
    revoke all on events.audit_chain from public cascade;
    for holder in
        select distinct a.grantee
        from pg_catalog.pg_class c
        cross join lateral pg_catalog.aclexplode(c.relacl) a
        where c.oid = 'events.audit_chain'::pg_catalog.regclass
            and a.grantee operator(pg_catalog.<>) c.relowner
            and a.grantee operator(pg_catalog.<>) 0
    loop
        execute pg_catalog.format(
            'revoke all on events.audit_chain from %I cascade',
            pg_catalog.pg_get_userbyid(holder)
        );
    end loop;
end;
$$;

grant select, insert on events.audit_chain to anima_runtime;
