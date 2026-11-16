-- The tool inventory of each device: its developer tools, its Rust build targets, its
-- processor architecture and its operating system version (K2 0710-0729; CAP-020 to CAP-031,
-- K8 Section 4; operator, 2026-11-15, CAP-020 to CAP-031; ruling number to be assigned).
--
-- `inventories` holds one row per device whose agent sent an inventory: the architecture, the
-- operating system, its version and when the hub stored it. `tools` holds one row per tool the
-- agent found, with its version, or no version when the probe failed (the tool is then not
-- usable). `build_targets` holds one row per installed Rust target. An agent batch with an
-- `inventory` replaces a device's rows together (DELETE then INSERT in one transaction), so a
-- reader sees the old inventory or the new one. The checks repeat the agent-core rules (K8
-- Section 4): text of 1 to 64 bytes of a fixed alphabet. A tool's name is not limited to the
-- names this version knows, so a later hub can add a tool without a migration; the hub stores
-- only names it knows.
--
-- Deleting a device deletes its inventory (DEV-015), and deleting an inventory deletes its tools
-- and targets.
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` SELECT,
-- INSERT, UPDATE and DELETE on the new tables. The migration only creates empty tables, so it is
-- cheap on a large database (DB-010).

create table metrics.inventories (
    device_id uuid not null,
    architecture text not null,
    os text not null,
    os_version text not null,
    found_at timestamptz not null,
    constraint inventories_pkey primary key (device_id),
    constraint inventories_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint inventories_architecture_check check (architecture in ('aarch64', 'x86_64')),
    constraint inventories_os_check check (os in ('macos', 'linux')),
    constraint inventories_os_version_check
        check (os_version ~ '^[0-9A-Za-z.+_-]{1,64}$')
);

create table metrics.tools (
    device_id uuid not null,
    name text not null,
    version text,
    constraint tools_pkey primary key (device_id, name),
    constraint tools_device_id_fkey foreign key (device_id)
        references metrics.inventories (device_id) on delete cascade,
    constraint tools_name_check check (name ~ '^[0-9a-z_.-]{1,64}$'),
    constraint tools_version_check check (version ~ '^[0-9A-Za-z.+_-]{1,64}$')
);

create table metrics.build_targets (
    device_id uuid not null,
    target text not null,
    constraint build_targets_pkey primary key (device_id, target),
    constraint build_targets_device_id_fkey foreign key (device_id)
        references metrics.inventories (device_id) on delete cascade,
    constraint build_targets_target_check check (target ~ '^[0-9a-z_.-]{1,64}$')
);
