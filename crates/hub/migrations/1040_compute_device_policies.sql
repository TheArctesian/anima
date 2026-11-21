-- The device policy of each compute node (CAP-010 to CAP-012; compute plan task L1-5). A new
-- range of K2, 1040-1059, schema `compute`: waits for a ruling, R-NEW-devicepolicy.
--
-- One row per device whose operator set a policy; a device with no row has the default policy
-- of the hub (75 percent memory, 50 percent disk, no quiet hours, mains power only, the
-- operator may be active). The switch "takes work" is not here: it is `jobs.machines` (USE-004,
-- OD-C8, R-165). The quiet hours are all three columns or none; they go past midnight when the
-- end is not after the start. No row holds a secret. Deleting a device deletes its row
-- (DEV-015).
--
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles"). The migration only creates an empty table (DB-010).
create schema compute;

create table compute.device_policies (
    device_id         uuid        primary key references devices.devices (id) on delete cascade,
    memory_share      smallint    not null check (memory_share between 1 and 100),
    disk_share        smallint    not null check (disk_share between 1 and 100),
    quiet_start       time,
    quiet_end         time,
    quiet_zone        text        check (char_length(quiet_zone) between 1 and 64),
    mains_only        boolean     not null,
    idle_only         boolean     not null,
    changed_at        timestamptz not null,
    check ((quiet_start is null) = (quiet_end is null)
       and (quiet_start is null) = (quiet_zone is null)),
    check (quiet_start is distinct from quiet_end or quiet_start is null)
);
