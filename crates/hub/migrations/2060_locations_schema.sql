-- The locations domain's three tables (K-E8 range 2060-2069; K-E4 Sections 11 and 12; R-145).
--
-- A sample is one observation of the position of the operator's phone. Its position is
-- sealed with the sealing key `locations` (Class B, VLT-003, R-146); its time is not, so that
-- a match with an event opens no other sample. `observed_at` is null when the provider gave
-- no time: such a sample never matches an event.
-- A transfer context holds what was known at the time of one transfer event. The IDs of the
-- event, the devices, the sites and the places are values: no foreign key leaves this schema,
-- and a later change of a site or of a place link does not change the row.
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles").
create schema locations;

create table locations.sample (
    id          uuid        primary key,
    provider    text        not null check (provider in ('dawarich', 'owntracks', 'anima')),
    account     text        not null check (length(account) between 1 and 200),
    sample_key  text        not null check (length(sample_key) between 1 and 200),
    observed_at timestamptz,
    received_at timestamptz not null,
    sealed      bytea       not null,
    unique (provider, account, sample_key)
);

create index sample_observed_at on locations.sample (observed_at);

create table locations.ingest_token (
    id           uuid        primary key,
    name         text        not null check (length(name) between 1 and 64),
    token_hash   bytea       not null unique,
    created_at   timestamptz not null,
    last_used_at timestamptz,
    revoked_at   timestamptz
);

create table locations.transfer_context (
    event_id           uuid        primary key,
    occurred_at        timestamptz not null,
    direction          text        not null check (direction in ('upload', 'download')),
    source_device      uuid,
    source_site        uuid,
    source_place       uuid,
    destination_device uuid,
    destination_site   uuid,
    destination_place  uuid,
    actor_kind         text        not null check (actor_kind in ('human', 'agent', 'system')),
    actor              text,
    client_device      uuid
);
