-- The placement of a job with its reasons (K2 range 0980-0999; task W3 part 3, R-259;
-- ROU-004 to ROU-007, PRO-051). No contract of the daemon changes: K10 Section 2 stays, a
-- machine asks and the hub never opens a connection to a machine. No row holds a secret.

-- The one machine that may get the job (ROU-006). Held as a value: a job outlives its device,
-- and a job that names a device that is gone goes to no different machine.
alter table jobs.jobs add column machine_id uuid;

-- What the last claim of a machine said, for the tie-break of the free memory, and the
-- exclusion of the operator: an excluded machine gets only work that names it (ROU-006).
-- `os` and `architecture` have no check: a claim with a word that this hub does not know
-- is stored as it is and fits no job.
alter table jobs.machines
    add column name              text    not null default '',
    add column excluded          boolean not null default false,
    add column types             text[]  not null default '{}',
    add column kinds             text[]  not null default '{}',
    add column os                text    not null default '',
    add column architecture      text    not null default '',
    add column free_slots        bigint  not null default 0 check (free_slots >= 0),
    add column free_memory_bytes bigint  not null default 0 check (free_memory_bytes >= 0),
    add column free_disk_bytes   bigint  not null default 0 check (free_disk_bytes >= 0);

-- The machine that the operator pinned for a project: each job of the project that names no
-- machine goes only to it (ROU-006).
create table jobs.project_machines (
    project_id uuid primary key,
    device_id  uuid not null references devices.devices (id) on delete cascade
);

-- The last decision of the placement for each job and each machine that asked (ROU-004):
-- `verdict` is `fits` or the one requirement that failed. `machine` is the name of the device
-- at that time, so that the reason can be read when the device is gone.
create table jobs.decisions (
    job_id            uuid        not null references jobs.jobs (id) on delete cascade,
    device_id         uuid        not null,
    machine           text        not null,
    at                timestamptz not null,
    verdict           text        not null check (verdict in (
        'fits', 'type', 'kind', 'os', 'architecture', 'memory', 'disk', 'refused_before',
        'names_other', 'excluded', 'waits_for_better'
    )),
    free_memory_bytes bigint      not null check (free_memory_bytes >= 0),
    free_disk_bytes   bigint      not null check (free_disk_bytes >= 0),
    primary key (job_id, device_id)
);
