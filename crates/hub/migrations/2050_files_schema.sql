-- The files domain (K-E8 range 2050-2059; K-E4 Sections 9 and 10; FIL-001 to FIL-005,
-- VIS-001 to VIS-004).
--
-- A piece is one file sealed with one data key; its `hash` is `<key_name>:<sha-256 hex>`, so the
-- same bytes under one key are one row and under two keys are two. The bytes are not in the
-- database: they are in the store (`ANIMA_FILES_DIR`). `owner` is a record reference
-- (`kind:id`); no foreign key leaves the schema. Privileges: migration 0001's default
-- privileges give `anima_runtime` what it needs on the new schema (K2 "Roles").
create schema files;

create table files.piece (
    hash        text        primary key check (hash ~ '^[a-z][a-z0-9_]{0,39}:[0-9a-f]{64}$'),
    key_name    text        not null check (key_name ~ '^[a-z][a-z0-9_]{0,39}$'),
    size        bigint      not null check (size between 0 and 268435456),
    sealed_size bigint      not null check (sealed_size > size),
    stored_at   timestamptz not null,
    check (hash = key_name || ':' || split_part(hash, ':', 2))
);

create table files.archive (
    id         uuid        primary key,
    owner      text        not null check (owner ~ '^[a-z][a-z0-9_]{0,39}:.' and length(owner) <= 241),
    name       text        not null check (name <> '' and length(name) <= 200),
    created_at timestamptz not null,
    created_by text        not null
);

create table files.attachment (
    id         uuid             primary key,
    hash       text             not null references files.piece,
    owner      text             not null check (owner ~ '^[a-z][a-z0-9_]{0,39}:.' and length(owner) <= 241),
    name       text             not null check (name <> '' and length(name) <= 200),
    media_type text             not null check (media_type ~ '^[a-z0-9!#$&^_.+-]+/[a-z0-9!#$&^_.+-]+$'),
    archive_id uuid             references files.archive,
    latitude   double precision check (latitude between -90 and 90),
    longitude  double precision check (longitude between -180 and 180),
    created_at timestamptz      not null,
    created_by text             not null,
    check ((latitude is null) = (longitude is null))
);

create index attachment_owner on files.attachment (owner);
create index attachment_archive on files.attachment (archive_id);
create index attachment_hash on files.attachment (hash);

-- `fields` is the list of typed fields: as the extraction gave them while `state` is
-- `proposed`, as the operator left them once `confirmed` (VIS-001, VIS-004).
create table files.proposal (
    id            uuid        primary key,
    attachment_id uuid        references files.attachment on delete cascade,
    owner         text        not null check (owner ~ '^[a-z][a-z0-9_]{0,39}:.' and length(owner) <= 241),
    form          text        not null check (form ~ '^[a-z][a-z0-9_]{0,39}$'),
    origin        text        not null check (origin in ('client', 'hub')),
    fields        jsonb       not null check (jsonb_typeof(fields) = 'array'),
    state         text        not null check (state in ('proposed', 'confirmed', 'rejected')),
    created_at    timestamptz not null,
    decided_at    timestamptz,
    decided_by    text,
    check ((state = 'proposed') = (decided_at is null)),
    check ((decided_at is null) = (decided_by is null))
);

create index proposal_owner on files.proposal (owner, state);
create index proposal_attachment on files.proposal (attachment_id);
