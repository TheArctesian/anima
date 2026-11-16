-- The links domain's one table (K-E8 range 2030-2039; LNK-001, LNK-002).
--
-- `source` and `target` are record references (`kind:id`, K-E4). No foreign key: the record
-- of either side belongs to another domain and may not exist yet. `label` is the text the
-- link shows while its target has no record (LNK-002). Privileges: migration 0001's default
-- privileges give `anima_runtime` what it needs on the new schema (K2 "Roles").
create schema links;

create table links.record_link (
    id         uuid        primary key,
    source     text        not null check (source ~ '^[a-z][a-z0-9_]{0,39}:.' and length(source) <= 241),
    target     text        not null check (target ~ '^[a-z][a-z0-9_]{0,39}:.' and length(target) <= 241),
    kind       text        not null check (kind ~ '^[a-z][a-z0-9_]{0,39}$'),
    label      text        check (label <> '' and length(label) <= 200),
    created_at timestamptz not null,
    created_by text        not null,
    unique (source, target, kind),
    check (source <> target)
);

create index record_link_target on links.record_link (target);
