-- One rule for GPU series from SSH and from the agent, and the Apple GPU's memory kind (K2
-- 0710-0729; MET-007, MET-009, K8 Section 4; R-092 D-6, R-099).
--
-- A GPU-kind series (`gpu_percent`, `vram_percent`, `vram_bytes`, `gpu_temperature_c`,
-- `gpu_power_w`, `gpu_memory_bytes`) stores `gpu_index` = its GPU and `label` = that index as
-- decimal text; every other kind has `gpu_index` null. The 0700 unique key (device_id, kind,
-- label) stays and keeps two GPUs of one device apart (R-099). The agent's ingest derives the
-- label from the index; these checks make the database refuse any row that breaks the rule,
-- whichever writer sends it.
--
-- `gpu_memory_bytes` is the memory in use by a unified-memory GPU (Apple), never a VRAM value
-- (R-092 D-6). It joins the kind check in the same change as the hub's `MetricKind` member, so
-- a stored row of it is always readable.
--
-- Only checks change. Each scans `metrics.series`, which holds one row per device, kind and
-- label, not the samples, so the migration is cheap on a large database (DB-010). The new kind
-- check admits every old value.

alter table metrics.series drop constraint series_kind_check;

alter table metrics.series
    add constraint series_kind_check check (kind in (
        'cpu_percent', 'memory_percent', 'memory_bytes', 'disk_percent', 'disk_bytes',
        'network_in_bps', 'network_out_bps', 'uptime_seconds', 'load_1', 'load_5', 'load_15',
        'gpu_percent', 'vram_percent', 'vram_bytes', 'gpu_temperature_c', 'gpu_power_w',
        'gpu_memory_bytes'
    ));

alter table metrics.series
    add constraint series_gpu_kind_check check (
        (gpu_index is not null) = (kind in (
            'gpu_percent', 'vram_percent', 'vram_bytes', 'gpu_temperature_c', 'gpu_power_w',
            'gpu_memory_bytes'
        ))
    );

alter table metrics.series
    add constraint series_gpu_label_check check (
        gpu_index is null or label = gpu_index::text
    );
