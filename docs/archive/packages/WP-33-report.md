# WP-33 report — Client detail view and charts

| Item | Value |
| --- | --- |
| Status | **Done** |
| Package branch | `claude/wp-33-detail-charts` (head in the final commit of this report) |
| Base | `phase-2` d98544b; rebased after each programme merge, last on 0021e308 |
| Plan | [WP-33.md](WP-33.md) |
| Gates | 4–12 on every task (R-068). Gates 1–3 were not run because no Rust, Cargo or contract file changed. |

## Delivered

| Task | Accepted | What |
| --- | --- | --- |
| T1 charts | 672e1887 | `lib/charts/`: scales, plot (gaps, min–max band, threshold), ticks with a per-tick zone offset, label, `LineChart`, `ChartSkeleton`, `ChartFrame`. Chart update slide on `spring-default` from the on-screen position, `translate` only, instant under reduced motion. A range change cross-fades. The checks response chart runs on it, and `checks/chart.ts` is gone. |
| T2 metrics | 7d4d2b8d | `lib/metrics/`: `api.ts` (DeviceMetrics, MetricsAdded), `charts.ts` (chart per metric, per mount, per interface, per GPU; VRAM only with `hasVram`; "GPU <index> <name>" titles), `units.ts`, `live.ts` (`mergeLive`), and later `ranges.ts`. |
| T3 bars | bf40934e | `lib/metrics/bars/`: WP-26's bar row moved without rewriting, typed structurally (`BarReadings`). |
| T4 overview | 4a236a48 | Overview metrics section: range control in the URL (`?range=day|week`), current bars, chart grid, gap per range, live 1-hour points, empty, error and skeleton states, per-tab skeleton in `TabPanel`. |
| T5 sessions | ee585288 | Protocol column in the Sessions tab (RA-030). |
| T6 skeleton-key | a49a195a | The Overview skeleton reserves the host key's wrapped height (MOT-007). |
| T7 package-fixes | ef88c3eb | Package review: the label and gap follow the range on screen (`shownRange`); range helpers in one file; dead exports removed; stale `@pending` comment removed. |

Interface measurement (one per package, R-066): Playwright WebKit and Chromium, 600, 900 and 1400 px, fakeHub with synthetic data, API delayed 400 ms.
- Final results: CLS 0. Skeleton-to-content offset 0.00 px. Skeleton and content opacities sum to 1 through the swap.
- The slide continues from the on-screen value when interrupted. Only `translate` and `opacity` animate. Nothing repeats.
- Reduced motion is instant. Axis labels fit their column.
- Series 2 and 3 have dash and dot patterns. The pressed range button has an underline.

Criterion 13, client side: five tabs; charts for 1 hour, 24 hours and 7 days; a usage chart per GPU and a VRAM chart only with VRAM, titled with index and name. The device header (DEV-020–022) belongs to WP-26.

## Edits outside owned paths

- `apps/client/src/routes/devices/[id]/+page.svelte`: wiring only (D-6). It creates `DeviceMetrics`, loads it on the Overview tab and when the range changes, stops it on leave or destroy, and passes the skeleton snippet and the callbacks.
- `apps/client/src/routes/devices/[id]/page.metrics.test.ts` (new). The folder is now at 12 files, which is the limit.
- `apps/client/src/lib/hub/generated/`: regenerated with `bun run graphql` only.

## Contract change requests

None. One token request (could): K5 needs an opacity token for the chart min–max band. It uses `fill-opacity: 0.2` today (`lib/charts/LineChart.svelte`).

## Handoffs

- **H-1 to the `lib/devices` owner (WP-26), relayed by the programme orchestrator.**
  1. In `lib/devices/card/DeviceCard.svelte`, import `MetricBars` from `$lib/metrics/bars/MetricBars.svelte` and render `<MetricBars readings={device.readings ?? null} source={device.metricsSource} />`.
  2. Map `--device-card-readings-block-size` to `--metric-bars-block-size` and `--device-card-readings-label-block-size` to `--metric-bars-label-block-size`, including in `lib/devices/card/geometry.css`. The values are identical.
  3. Delete `lib/devices/readings/`. Until then, `DEGRADED_ABOVE` and `percentText` exist twice.
  4. `endTransitionsAtOnce` is copied in `lib/metrics/bars/bars.fixture.ts` and also lives in `lib/devices/transitions.fixture.ts`. One shared copy belongs under `lib/motion/`, which is a programme or motion-owner change.
- **H-2 to the `lib/sessions` owner (WP-24 or WP-32).** `Session.clientAddr` (RA-034) is served by the hub once WP-22 lands it. The Sessions tab can show it after `DeviceSessions` selects it (D-10).

## Coulds (for issue #1)

- Opacity token for the chart band (see above).
- Host uptime from `UPTIME_SECONDS` is fetched but not shown. `formatUptime` was removed; add it back with a fact row if wanted.
- The chart skeleton draws four cells, while a device may have 5–9 or more charts. The grid is the last block, so CLS is 0, but the page grows at the bottom on arrival. The skeleton could reserve the last-known chart count.
- Time labels jump one step on a chart update while the line slides. The spec says axes do not move; a label cross-fade could be added.
- Weekday tick labels are fixed English; `Intl.DateTimeFormat` could be used.
- When the clocks go back, an hourly tick in the repeated hour gets one label, not two.
- 5.7.3 says the line "moves left continuously". D-2 does one spring slide per new point, because a continuous drift would be ambient motion (MOT-011). The WP-34 motion audit should confirm this reading.
- Live points update only the 1-hour range. 24 hours and 7 days reload on a range change.
- A stale hub `readings` answer can briefly replace a newer live reading during a load.
- A test for colour wrap past six series (seven disks) through `chartsOf`.
- Under `prefers-contrast: more` the chart grid stays `chart-grid` (nord3), not `text-muted` (MAT-006).
- Sessions Start and End timestamps wrap at 600 px.
- `MetricsSection.svelte:5` has a four-space indent on one import.

## Open risks

- `Device.gpus` returns `[]` until GPU discovery is built (WP-28 D-1). GPU charts are tested only on synthetic data.
- `routes/devices/[id]/` is at the 12-file limit. The next file there must go into a subfolder.
- No live run against the real hub yet. All measurement used fakeHub with K6-shaped synthetic data.
