-- The jobs of the work route (K2 range 0980-0999; contract K10, `contracts/work-route.md`,
-- accepted by R-209; PRO-050, PRO-051; task W3). A job is what K10 calls an item of work: the
-- hub gives it to the daemon of a machine, which holds it by a lease.
--
-- `project_id` and each device id but the one of `machines` and `faults` are held as values:
-- a job outlives its project and the device that ran it. No row holds a secret.
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles").
create schema jobs;

create table jobs.jobs (
    id                   uuid        primary key,
    work_type            text        not null check (work_type in ('build', 'runner')),
    kind                 text        not null check (kind in ('container', 'native')),
    -- What the machine must be; null takes each.
    os                   text        check (os in ('linux', 'macos')),
    architecture         text        check (architecture in ('x86_64', 'aarch64')),
    project_id           uuid,
    forge                text        not null check (forge in ('github')),
    owner                text        not null check (owner ~ '^[A-Za-z0-9-]{1,39}$'),
    name                 text        not null check (name ~ '^[A-Za-z0-9._-]{1,100}$'),
    parameters           jsonb       not null,
    -- The purposes of the secrets that the job needs (K10 Section 8). Never a value.
    secrets              text[]      not null default '{}',
    limit_seconds        bigint      not null check (limit_seconds > 0),
    limit_idle_seconds   bigint      not null check (limit_idle_seconds > 0),
    limit_memory_bytes   bigint      not null check (limit_memory_bytes > 0),
    limit_disk_bytes     bigint      not null check (limit_disk_bytes > 0),
    limit_orphan_seconds bigint      not null check (limit_orphan_seconds > 0),
    state                text        not null check (state in (
        'queued', 'claimed', 'preparing', 'running', 'finishing', 'done', 'failed', 'stopped'
    )),
    -- The number of the attempt that holds or last held the job; 0 before the first claim.
    attempt              integer     not null default 0 check (attempt >= 0),
    -- Leases that ended with no report (BLD-006).
    lost                 integer     not null default 0 check (lost between 0 and 3),
    device_id            uuid,
    last_device_id       uuid,
    -- The machines that refused the job: it is not given to them again (K10 Section 5).
    refused_by           uuid[]      not null default '{}',
    lease_ends_at        timestamptz,
    claimed_at           timestamptz,
    -- A stop that the hub tells the holder in its next answer (K10 Section 7).
    stop_reason          text        check (stop_reason in (
        'operator', 'runners_off', 'update', 'superseded', 'expired'
    )),
    -- Why the job failed or stopped: a fixed code or a fixed sentence of the hub.
    reason               text        check (octet_length(reason) <= 200),
    -- The result that the holder reported: fixed codes and numbers (K10 Section 7).
    result               jsonb,
    -- The count of bytes of the log that the hub accepted.
    log_end              bigint      not null default 0 check (log_end >= 0),
    created_at           timestamptz not null,
    state_changed_at     timestamptz not null,
    ended_at             timestamptz,
    constraint jobs_lease_check check (
        (state in ('claimed', 'preparing', 'running', 'finishing')) = (lease_ends_at is not null)
    ),
    constraint jobs_holder_check check (
        (state in ('claimed', 'preparing', 'running', 'finishing')) = (device_id is not null)
    ),
    constraint jobs_ended_check check ((state in ('done', 'failed', 'stopped')) = (ended_at is not null))
);

create index jobs_queue on jobs.jobs (created_at, id) where state = 'queued';
create index jobs_leases on jobs.jobs (lease_ends_at) where lease_ends_at is not null;
create index jobs_holder on jobs.jobs (device_id) where device_id is not null;

-- The log of a job, in the parts that the holder sent, after the hub's redaction. `from_byte`
-- is the count of bytes of the holder's log before the part. The hub keeps the newest 2 MiB.
create table jobs.log_parts (
    job_id    uuid   not null references jobs.jobs (id) on delete cascade,
    from_byte bigint not null check (from_byte >= 0),
    text      text   not null check (octet_length(text) <= 131072),
    primary key (job_id, from_byte)
);

-- What the hub knows of a machine for work: what its last claim said, and if the hub gives it
-- work. `off_by_operator` is the switch in the hub (USE-004); `off_until` is the hour with no
-- work after 5 jobs that the machine refused or lost in 1 hour (K10 Section 11).
create table jobs.machines (
    device_id       uuid        primary key references devices.devices (id) on delete cascade,
    switch          text        not null check (switch in ('on', 'off')),
    unclean         boolean     not null,
    off_by_operator boolean     not null default false,
    off_until       timestamptz,
    first_on_at     timestamptz,
    last_claim_at   timestamptz not null
);

-- One row for each job that a machine refused or lost.
create table jobs.faults (
    device_id uuid        not null references devices.devices (id) on delete cascade,
    at        timestamptz not null,
    kind      text        not null check (kind in ('refused', 'lost'))
);

create index faults_recent on jobs.faults (device_id, at);
