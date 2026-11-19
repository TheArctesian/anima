-- Operational keys: the credentials and runtime secrets a server of the operator needs, with
-- their versions and the servers that may fetch them (R-147; K-E5 Section 2; K2 range 2070-2079).
--
-- `sealed` is the one-column layout of `credentials::Sealed` (SEC-003), sealed under the AEAD
-- context `credentials.operational_key_versions:<version id>`, so a value copied to another
-- version row does not open. No other column holds a value (LOG-007). 30 days after a version
-- ended, the hub replaces its sealed value with empty bytes. A version that ended is never
-- active again. Privileges come from the default grants of 0001.

create table credentials.operational_keys (
    id            uuid        primary key,
    name          text        not null unique check (name ~ '^[a-z][a-z0-9_.-]{0,79}$'),
    provider      text        not null check (provider <> '' and length(provider) <= 200),
    purpose       text        not null check (purpose <> '' and length(purpose) <= 200),
    instructions  text        not null default '' check (length(instructions) <= 4000),
    provider_link text        null check (provider_link ~ '^https://' and length(provider_link) <= 2000),
    rotation_days integer     null check (rotation_days between 1 and 3650),
    expires_at    timestamptz null,
    created_at    timestamptz not null,
    created_by    text        not null,
    removed_at    timestamptz null
);

create table credentials.operational_key_versions (
    id                  uuid        primary key,
    key_id              uuid        not null references credentials.operational_keys (id),
    number              integer     not null check (number >= 1),
    sealed              bytea       not null,
    state               text        not null check (state in ('active', 'superseded', 'revoked', 'expired')),
    created_at          timestamptz not null,
    created_by          text        not null,
    ended_at            timestamptz null,
    rotated_at_provider boolean     not null default false,
    unique (key_id, number),
    check ((state = 'active') = (ended_at is null))
);

-- At most one active version for each key (R-147: no overlap).
create unique index operational_key_versions_one_active
    on credentials.operational_key_versions (key_id) where state = 'active';

-- A device's consumer rows go with the device (K-E5 Section 2.1, "After removal").
create table credentials.operational_key_consumers (
    key_id          uuid        not null references credentials.operational_keys (id),
    device_id       uuid        not null references devices.devices (id) on delete cascade,
    workload        text        not null check (workload ~ '^[a-z][a-z0-9_.-]{0,39}$'),
    granted_at      timestamptz not null,
    granted_by      text        not null,
    held_version    integer     null,
    held_seen_at    timestamptz null,
    fetched_version integer     null,
    fetched_at      timestamptz null,
    primary key (key_id, device_id, workload)
);

create index operational_key_consumers_device on credentials.operational_key_consumers (device_id);
