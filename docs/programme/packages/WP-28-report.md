# WP-28 report — Hub metrics

Status: **done**. T1 d7db9e4, T2 48d9ef0, T3 6d4b72b7 and T4 00e4ca8d are accepted (12/12 each, no must)
and merged into phase-2 (T4 as b4f5d70). Final package review: accept.

## What changed

- New domain `crates/hub/src/metrics/`:
  - pure readers for Linux `/proc` + `df -P -B1`, and for macOS `sysctl`, `vm_stat`, `df -P -k`,
    `netstat -ib -n` and `top -l 2`
  - rates (D-4)
  - a static command table (MET-002, CHK-012)
  - a collector every 30 s: 8 devices at once, 20 s per device, skipped ticks, links via
    `sessions::link` (P2-003, P2-004), Windows/Other skipped (MET-005)
  - its own broadcast `Feed` (D-15)
  - faces `readings`, `series`, `gpus`, and the wiring-only `keep_partitions`
- Migration `0700_metrics_schema.sql`:
  - `series`
  - `samples`, partitioned by day
  - `minute`/`hour`/`day` aggregates, written in the same transaction
  - `gpus` and `discovery`
  - SECURITY DEFINER partition functions
  - retention 30 d / 7 d / 90 d / 2 y as a `Retention` value (HUB-005), with orphan samples and idle
    series deleted (DEV-015, D-5)
- `Subscription.metricsAdded(deviceId)`, behind `Authenticated`.
- Programme files (PKG-001, R-072): `lib.rs`, `schema.rs`, `main.rs`, the new wiring module `upkeep.rs`,
  `quality/hub-modules.json`, `tests/it/hub.rs`, and `@pending` removed from the ten metrics names.

## Handoffs

- **H-1 events:** add `Source::Metrics` (K1 `EventSource.METRICS`). The collector uses `Source::Device`
  until then (D-10).
- **H-2 devices (`devices/graphql.rs`):** resolve `Device.readings` (`metrics::readings`, batched),
  `Device.metrics(range)` (`metrics::series`) and `Device.gpus` (`metrics::gpus`). Then drop their
  `@pending` and the three `register_output_type` lines in `schema.rs`.
- **WP-33 (charts):** `MetricSeries` points are HOUR = 60 one-minute points, DAY = 144 ten-minute points,
  WEEK = 168 hourly points. Each point has `min`/`max`. Bytes are Float. Live updates come from
  `metricsAdded(deviceId)`, and `Device.metrics` waits for H-2. There are no GPU series yet (C-1).
- **WP-26 (list bars):** `Readings` gives `cpuPercent`, `memoryPercent`, `diskPercent` (fullest mount)
  within 2 minutes; an older reading means none. `vramPercent`/`vramGpu` stay null (C-1). The card
  field waits for H-2; `metricsAdded.readings` already carries it.

## Open coulds (issue #1)

- C-1: GPU discovery and metrics (MET-006–MET-010). The tables and the `gpus` face exist.
- C-2: narrow the `metrics` public surface to `pub(crate)` where nothing outside uses it.
- C-3: add a metrics test that a deny-listed device gets no samples (P2-004 is covered in `sessions::link`).
- C-4: rename the `met_005_*` subscription tests and fix the stale `main.rs` doc in `metrics.rs`.
- C-5: test the guard and filter mutants on `metricsAdded` (the run did not finish on the lock).
- C-6: the shared target dir made stale test binaries possible; runs now `touch` sources under the lock.
