-- Agent tokens and enrolment codes (K2 0710-0729, K8 Sections 2, 3 and 5; MET-011 to MET-013,
-- R-092 D-1, D-2).
--
-- Both tables hold only SHA-256 hashes of the 32-byte token and the 16-byte code (MET-012,
-- MET-013): a database reader learns no value an agent or an operator can present. Each row
-- belongs to one device and goes with it (on delete cascade), so a deleted device takes its
-- token and its codes with it. An agent token is never an `access.api_tokens` row, so it can
-- never pass the GraphQL sign-in (R-092 D-1).
--
-- `agents.next_*` is the renewed token, kept beside the current one until its first use
-- promotes it (K8 Section 5). `last_batch_at` is the time of the last accepted batch, for the
-- stale mark (MET-018); the push route writes it.
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` SELECT,
-- INSERT, UPDATE and DELETE on both tables. Only new tables and one check change, so the
-- migration is cheap on a large database (DB-010).

create table metrics.agents (
    device_id uuid not null,
    token_sha256 bytea not null,
    expires_at timestamptz not null,
    next_token_sha256 bytea null,
    next_expires_at timestamptz null,
    last_batch_at timestamptz null,
    constraint agents_pkey primary key (device_id),
    constraint agents_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint agents_token_sha256_key unique (token_sha256),
    constraint agents_next_token_sha256_key unique (next_token_sha256),
    constraint agents_token_sha256_check check (octet_length(token_sha256) = 32),
    constraint agents_next_token_sha256_check check (octet_length(next_token_sha256) = 32),
    -- A renewed token always has its own expiry, and an expiry never stands alone.
    constraint agents_next_check check ((next_token_sha256 is null) = (next_expires_at is null))
);

create table metrics.enrolments (
    code_sha256 bytea not null,
    device_id uuid not null,
    expires_at timestamptz not null,
    used_at timestamptz null,
    constraint enrolments_pkey primary key (code_sha256),
    constraint enrolments_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint enrolments_code_sha256_check check (octet_length(code_sha256) = 32)
);

-- A new code voids the device's unused ones, and a device delete cascades: both find the
-- device's codes by this index.
create index enrolments_device_id_idx on metrics.enrolments (device_id);

-- MET-011: `agent` joins the metrics sources. Only enrolment sets it (R-092 D-7). A metrics
-- migration changes the devices check as 0701 changed the events check (K2 0710-0729). 0110
-- named the check; the drop and add hold the devices table's lock for the length of the
-- migration, and the scan is short, because the new check admits every old value.
alter table devices.devices drop constraint devices_metrics_source_check;

alter table devices.devices
    add constraint devices_metrics_source_check
    check (metrics_source in ('none', 'ssh', 'agent'));
