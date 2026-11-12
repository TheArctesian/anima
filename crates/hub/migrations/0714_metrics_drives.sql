-- The drives of a device and the activity series of each (K2 0710-0729; MET-027, MET-028,
-- K8 Section 4; operator ruling 2026-11-12).
--
-- `drives` holds one row per drive the device's `lsblk` listed: its kernel name, its kind (from
-- the transport and the rotation), its size and the operator's mount points on it, empty when it
-- is not mounted. The SSH collector and the agent's push replace a device's rows together when
-- the list changes (DELETE then INSERT in one transaction), so a reader sees the old list or the
-- new one. The checks repeat the agent-core rules (K8 Section 4): a name of 1 to 128 bytes, at
-- most 32 mounts, a size above zero. A drive that leaves the list keeps its series.
--
-- The three drive kinds join the series kind check in the same change as the hub's
-- `MetricKind` members, so a stored row of them is always readable. They are labelled with the
-- drive name and have no GPU, which 0711's checks already require of every non-GPU kind.
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` SELECT,
-- INSERT, UPDATE and DELETE on the new table. The kind check scans `metrics.series`, one row per
-- device, kind and label, not the samples, and admits every old value, so the migration is cheap
-- on a large database (DB-010).

alter table metrics.series drop constraint series_kind_check;

alter table metrics.series
    add constraint series_kind_check check (kind in (
        'cpu_percent', 'memory_percent', 'memory_bytes', 'disk_percent', 'disk_bytes',
        'network_in_bps', 'network_out_bps', 'uptime_seconds', 'load_1', 'load_5', 'load_15',
        'gpu_percent', 'vram_percent', 'vram_bytes', 'gpu_temperature_c', 'gpu_power_w',
        'gpu_memory_bytes', 'disk_read_bps', 'disk_write_bps', 'disk_busy_percent'
    ));

create table metrics.drives (
    device_id uuid not null,
    name text not null,
    kind text not null,
    size_bytes bigint not null,
    mounts text[] not null default '{}',
    found_at timestamptz not null,
    constraint drives_pkey primary key (device_id, name),
    constraint drives_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint drives_name_check check (octet_length(name) between 1 and 128),
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint drives_kind_check check (kind in ('nvme', 'ssd', 'hdd', 'disk')),
    constraint drives_size_bytes_check check (size_bytes > 0),
    constraint drives_mounts_check check (cardinality(mounts) <= 32)
);
