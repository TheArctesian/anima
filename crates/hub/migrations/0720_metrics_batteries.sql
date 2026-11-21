-- The batteries of a device and the charge series of each (K2 0710-0729; MET-032, CAP-004,
-- K8 Section 4).
--
-- `batteries` holds one row per battery that powers the device, as its daemon last listed it:
-- its name, its state, the time left while it charges or discharges, and its health. The push
-- replaces a device's rows together when a batch carries `batteries` (DELETE then INSERT in one
-- transaction), so a reader sees the old list or the new one. The checks repeat the K8 rules: a
-- name of 1 to 128 bytes, a known state, a time left of at least 0 and a health of 0 to 100. A
-- battery that leaves the list keeps its series.
--
-- The kind `battery_percent` joins the series kind check in the same change as the hub's
-- `MetricKind` member, so a stored row of it is always readable. It is labelled with the
-- battery name and has no GPU, which 0711's checks already require of every non-GPU kind.
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
        'gpu_memory_bytes', 'disk_read_bps', 'disk_write_bps', 'disk_busy_percent',
        'battery_percent'
    ));

create table metrics.batteries (
    device_id uuid not null,
    name text not null,
    state text not null,
    seconds_left integer null,
    health_percent smallint null,
    found_at timestamptz not null,
    constraint batteries_pkey primary key (device_id, name),
    constraint batteries_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint batteries_name_check check (octet_length(name) between 1 and 128),
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint batteries_state_check check (
        state in ('charging', 'discharging', 'full', 'not_charging', 'unknown')
    ),
    constraint batteries_seconds_left_check check (seconds_left >= 0),
    constraint batteries_health_percent_check check (health_percent between 0 and 100)
);
