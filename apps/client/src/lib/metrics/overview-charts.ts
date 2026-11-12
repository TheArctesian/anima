import type { ChartPoint, ChartSeries, SeriesColour } from '$lib/charts/plot';
import type { ValueAxis } from '$lib/charts/scales';
import type { DeviceGpu, MetricKind, MetricSeries } from './api';
import { DEGRADED_ABOVE, formatBytes, formatBytesPerSecond, formatLoad, formatPercent } from './units';

/** K5: `chart-series-1` to `-6`. */
const SERIES_COLOURS = 6;

/** One chart of the Overview (D-4). Its fields are the props of `LineChart` except `span`, which the range gives. */
export interface Chart {
  /** Stable across reloads and live points: use it as the keyed-each key. */
  readonly key: string;
  readonly title: string;
  readonly series: readonly ChartSeries[];
  readonly axis: ValueAxis;
  /** MET-004: the 85 % line on percent charts only. */
  readonly threshold?: number;
  readonly format: (value: number) => string;
}

/** The colour of the series at `position` (0-based): 1 to 6, then round again. */
function seriesColour(position: number): SeriesColour {
  return ((position % SERIES_COLOURS) + 1) as SeriesColour;
}

/** The hub's points as chart points: `at` in milliseconds, oldest first; a point with an unreadable time is dropped. */
function pointsOf(series: MetricSeries): ChartPoint[] {
  return series.points
    .map((point) => ({ at: Date.parse(point.at), value: point.value, min: point.min, max: point.max }))
    .filter((point) => Number.isFinite(point.at))
    .sort((a, b) => a.at - b.at);
}

/** A line of the chart from each pair of a legend name and the hub series that feeds it; a missing series gives no line. */
function lines(pairs: readonly (readonly [string, MetricSeries | undefined])[]): ChartSeries[] {
  const found = pairs.flatMap(([label, series]) => (series ? [{ label, series }] : []));
  return found.map(({ label, series }, position) => ({ label, colour: seriesColour(position), points: pointsOf(series) }));
}

function percentChart(key: string, title: string, label: string, series: MetricSeries | undefined): Chart[] {
  if (!series) return [];
  const drawn = lines([[label, series]]);
  return [{ key, title, series: drawn, axis: 'percent', threshold: DEGRADED_ABOVE, format: formatPercent }];
}

function openChart(key: string, title: string, format: Chart['format'], drawn: readonly ChartSeries[]): Chart[] {
  return drawn.length === 0 ? [] : [{ key, title, series: drawn, axis: 'open', format }];
}

/** The first series of `kind` that is not a GPU's; cpu, memory and load carry an empty or fixed label we do not show (WP-28 D-5). */
function first(all: readonly MetricSeries[], kind: MetricKind): MetricSeries | undefined {
  return all.find((series) => series.kind === kind && series.gpuIndex === null);
}

function byLabel(all: readonly MetricSeries[], kind: MetricKind): MetricSeries[] {
  return all.filter((series) => series.kind === kind && series.gpuIndex === null).sort((a, b) => a.label.localeCompare(b.label));
}

function diskCharts(all: readonly MetricSeries[]): Chart[] {
  return byLabel(all, 'DISK_PERCENT').flatMap((mount) => percentChart(`disk:${mount.label}`, `Disk ${mount.label}`, mount.label, mount));
}

/** One chart of two rates per label, ordered by label: a network interface's in and out, a drive's read and write. */
function pairCharts(all: readonly MetricSeries[], title: 'Drive' | 'Network', kinds: readonly [MetricKind, MetricKind], names: readonly [string, string]): Chart[] {
  const one = byLabel(all, kinds[0]);
  const other = byLabel(all, kinds[1]);
  const labels = [...new Set([...one, ...other].map((series) => series.label))].sort((a, b) => a.localeCompare(b));
  return labels.flatMap((label) => {
    const drawn = lines([
      [names[0], one.find((series) => series.label === label)],
      [names[1], other.find((series) => series.label === label)],
    ]);
    return openChart(`${title.toLowerCase()}:${label}`, `${title} ${label}`, formatBytesPerSecond, drawn);
  });
}

function loadChart(all: readonly MetricSeries[]): Chart[] {
  const pairs = [
    ['1 min', 'LOAD_1'],
    ['5 min', 'LOAD_5'],
    ['15 min', 'LOAD_15'],
  ] as const;
  return openChart('load', 'Load', formatLoad, lines(pairs.map(([name, kind]) => [name, first(all, kind)] as const)));
}

function gpuCharts(all: readonly MetricSeries[], gpus: readonly DeviceGpu[]): Chart[] {
  return [...gpus]
    .sort((a, b) => a.index - b.index)
    .flatMap((gpu) => {
      const title = `GPU ${String(gpu.index)} ${gpu.name}`;
      const ofGpu = (kind: MetricKind) => all.find((series) => series.kind === kind && series.gpuIndex === gpu.index);
      const usage = percentChart(`gpu:${String(gpu.index)}:usage`, `${title} usage`, 'Usage', ofGpu('GPU_PERCENT'));
      // MET-007: no VRAM chart for a GPU without its own memory, even if a series for it arrives.
      const vram = gpu.hasVram ? percentChart(`gpu:${String(gpu.index)}:vram`, `${title} VRAM`, 'VRAM', ofGpu('VRAM_PERCENT')) : [];
      // D-30: a unified-memory GPU (Apple) shows the memory it uses as bytes; a GPU with its own memory has VRAM, so nothing is "shared".
      const shared = gpu.hasVram
        ? []
        : openChart(`gpu:${String(gpu.index)}:memory`, `${title} memory in use (shared)`, formatBytes, lines([['Memory in use', ofGpu('GPU_MEMORY_BYTES')]]));
      return [...usage, ...vram, ...shared];
    });
}

/**
 * The charts the Overview draws, in a fixed order: CPU, memory, each disk mount (by mount point), each drive's
 * read and write (by drive name, MET-023), each network interface (in then out), load (1, 5, 15), then per GPU by index its usage, its VRAM and the shared memory of a unified-memory GPU (MET-007, MET-009, D-30).
 * Uptime is a fact, not a chart. A chart is listed only when at least one of its series exists, so a device
 * that reports no load, or a GPU with no samples yet, shows no empty frame; a series that exists with no
 * points still gets its chart (the Overview shows its empty state).
 */
export function chartsOf(series: readonly MetricSeries[], gpus: readonly DeviceGpu[]): readonly Chart[] {
  return [
    ...percentChart('cpu', 'CPU', 'CPU', first(series, 'CPU_PERCENT')),
    ...percentChart('memory', 'Memory', 'Memory', first(series, 'MEMORY_PERCENT')),
    ...diskCharts(series),
    ...pairCharts(series, 'Drive', ['DISK_READ_BPS', 'DISK_WRITE_BPS'], ['Read', 'Write']),
    ...pairCharts(series, 'Network', ['NETWORK_IN_BPS', 'NETWORK_OUT_BPS'], ['In', 'Out']),
    ...loadChart(series),
    ...gpuCharts(series, gpus),
  ];
}
