-- The sealed records domain (K-E8 range 2080-2089; K-E5 Section 3; VLT-001, VLT-002, VLT-006).
--
-- `sealed` and `sealed_index` are bytes that a client sealed with the vault key of Class A. The
-- hub has no key that opens them. `key_id` is the SHA-256 of that vault key. `owner` is a record
-- reference (`kind:id`, K-E4); no foreign key, because the record belongs to an extension.
-- `key_offer` carries a vault key from one client to a new one, sealed to their key exchange;
-- one read, 5 minutes. Privileges: migration 0001's default privileges give `anima_runtime`
-- what it needs on the new schema (K2 "Roles").
create schema sealed_records;

create table sealed_records.record (
    id           uuid        primary key,
    owner        text        not null unique check (owner ~ '^[a-z][a-z0-9_]{0,39}:.' and length(owner) <= 241),
    extension    text        not null check (extension ~ '^[a-z][a-z0-9_]{0,39}$'),
    sealed       bytea       not null check (octet_length(sealed) between 1 and 1048576),
    sealed_index bytea       check (octet_length(sealed_index) between 1 and 1048576),
    key_id       bytea       not null check (octet_length(key_id) = 32),
    created_at   timestamptz not null,
    updated_at   timestamptz not null
);

create table sealed_records.key_offer (
    id         uuid        primary key,
    sealed     bytea       not null check (octet_length(sealed) between 1 and 4096),
    created_at timestamptz not null,
    expires_at timestamptz not null
);
