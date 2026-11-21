-- The hardware values of each device (K2 0710-0729; CAP-001 to CAP-005, CAP-032, K8 Section 4;
-- R-165 approves the optional member `hardware` of the push batch; R-NEW-hardwaretable approves
-- this change of K2).
--
-- `hardware` holds one row per device whose daemon sent its hardware values: the system,
-- unified and dedicated memory, the usable memory of Apple silicon and its source, the free disk
-- space, the CPU features, one verdict per backend, the power source, the thermal state, whether
-- the operator is active and when the hub stored them. A value the daemon could not read is
-- null, never zero. `backend_names` and `backend_reasons` are parallel: a null reason is a usable
-- backend, else a fixed code (CAP-002), never text from the device. A daemon batch with a
-- `hardware` member replaces the row (DELETE then INSERT in the batch's transaction). The checks
-- repeat the daemon-core rules (K8 Section 4).
--
-- Deleting a device deletes its row (DEV-015).
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` SELECT,
-- INSERT, UPDATE and DELETE on the new table. The migration only creates an empty table, so it
-- is cheap on a large database (DB-010).

create table metrics.hardware (
    device_id uuid not null,
    system_memory_bytes bigint not null,
    unified_memory_bytes bigint,
    dedicated_memory_bytes bigint,
    usable_memory_bytes bigint,
    usable_memory_source text,
    free_disk_bytes bigint,
    cpu_features text[] not null,
    backend_names text[] not null,
    backend_reasons text[] not null,
    power_source text,
    thermal_state text,
    operator_active boolean,
    found_at timestamptz not null,
    constraint hardware_pkey primary key (device_id),
    constraint hardware_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint hardware_bytes_check check (
        system_memory_bytes >= 0 and unified_memory_bytes >= 0 and dedicated_memory_bytes >= 0
        and usable_memory_bytes >= 0 and free_disk_bytes >= 0),
    constraint hardware_usable_memory_check
        check ((usable_memory_bytes is null) = (usable_memory_source is null)),
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint hardware_usable_memory_source_check
        check (usable_memory_source in ('wired_limit', 'default_share')),
    constraint hardware_cpu_features_check check (cardinality(cpu_features) <= 32),
    constraint hardware_backends_check check (
        cardinality(backend_names) <= 16
        and cardinality(backend_names) = cardinality(backend_reasons)),
    constraint hardware_power_source_check check (power_source in ('mains', 'battery')),
    constraint hardware_thermal_state_check check (thermal_state in ('nominal', 'throttled'))
);
