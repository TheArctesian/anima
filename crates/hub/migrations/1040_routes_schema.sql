-- The routes domain's tables (range 1040-1059; HOST-046, R-NEW-hubrouting, task H21; Section 12
-- of docs/proposals/hosted-applications-stage-2.md).
--
-- A route is one public name of one application. Its target (the device, the port and, for a
-- path by the network, the address) is fixed when the name is added: a change is a removal and
-- a new route (SF-NEW-routetarget). The token of the proof is 256 random bits in hex, bound to
-- the pair of this hub and the name (SF-NEW-routetoken). `machine_consent` is the second
-- consent at the machine that OD-HOST-43 asks for: nothing can set it yet, so no route is
-- active (SF-NEW-routepublish). `application_recorded` says that the hub's record of the
-- application names this device and this port and no socket of the engine: the hub has no such
-- record before task H6, so nothing sets it and no route serves (SF-NEW-revroutetarget). The private key of a certificate is sealed with the hub key and
-- bound to its route (SEC-003); nothing in these tables is a body of a request (LOG-007).
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles").
create schema routes;

create table routes.route (
    id               uuid        primary key,
    name             text        not null unique
                         check (name ~ '^[a-z0-9]([a-z0-9.-]{0,251}[a-z0-9])?$'),
    application_id   uuid        not null,
    device_id        uuid        not null references devices.devices (id) on delete cascade,
    port             integer     not null check (port between 1 and 65535),
    address          inet,
    token            text        not null check (token ~ '^[0-9a-f]{64}$'),
    proof            text        not null default 'waiting'
                         check (proof in ('waiting', 'proven', 'lost')),
    failed_reads     integer     not null default 0 check (failed_reads >= 0),
    machine_consent  boolean     not null default false,
    application_recorded boolean not null default false,
    checked_at       timestamptz,
    issue_failures   integer     not null default 0 check (issue_failures >= 0),
    next_issue_at    timestamptz,
    created_at       timestamptz not null
);

create index route_application on routes.route (application_id);
create index route_device on routes.route (device_id);

create table routes.certificate (
    route_id    uuid        primary key references routes.route (id) on delete cascade,
    chain_pem   text        not null,
    sealed_key  bytea       not null,
    not_after   timestamptz not null,
    stored_at   timestamptz not null
);

create table routes.traffic (
    route_id    uuid        primary key references routes.route (id) on delete cascade,
    requests    bigint      not null default 0 check (requests >= 0),
    bytes_in    bigint      not null default 0 check (bytes_in >= 0),
    bytes_out   bigint      not null default 0 check (bytes_out >= 0),
    refused     bigint      not null default 0 check (refused >= 0),
    updated_at  timestamptz not null
);
