import type { DeviceGpu, MetricKind, MetricSeries } from './api';
import { formatBytesPerSecond, formatLoad, formatPercent, type UnitFormatter } from './units';

/** MET-004: percent charts mark this level. */
export const THRESHOLD_PERCENT = 85;
/** K5: `chart-series-1` to `-6`. */
const SERIES_COLOURS = 6;

export type SeriesColour = 1 | 2 | 3 | 4 | 5 | 6;

/** One line of a chart: which hub series feeds it, what the legend calls it, which `chart-series-N` colours it. */
export interface ChartSeries {
  readonly kind: MetricKind;
  /** The hub series label (mount point, interface name, "GPU <index> <name>", or empty). */
  readonly label: string;
  /** The legend text. */
  readonly name: string;
  readonly colour: SeriesColour;
}

export type ValueAxis =
  | { readonly kind: 'percent'; readonly min: 0; readonly max: 100; readonly threshold: typeof THRESHOLD_PERCENT }
  | { readonly kind: 'open'; readonly unit: 'bytesPerSecond' | 'load' };

/** One chart of the Overview (D-4). */
export interface Chart {
  /** Stable across reloads and live points: use it as the keyed-each key. */
  readonly key: string;
  readonly title: string;
  readonly series: readonly ChartSeries[];
  readonly axis: ValueAxis;
  readonly format: UnitFormatter;
}

const PERCENT_AXIS: ValueAxis = { kind: 'percent', min: 0, max: 100, threshold: THRESHOLD_PERCENT };

/** The colour of the series at `position` (0-based): 1 to 6, then round again. */
export function seriesColour(position: number): SeriesColour {
  return ((position % SERIES_COLOURS) + 1) as SeriesColour;
}

function lines(found: readonly MetricSeries[], names: readonly string[]): ChartSeries[] {
  return found.map((series, position) => ({
    kind: series.kind,
    label: series.label,
    name: names[position] ?? series.label,
    colour: seriesColour(position),
  }));
}

function ofKind(all: readonly MetricSeries[], kind: MetricKind): MetricSeries[] {
  return all.filter((series) => series.kind === kind && series.gpuIndex === null);
}

function percentChart(key: string, title: string, found: readonly MetricSeries[], names: readonly string[]): Chart[] {
  return found.length === 0 ? [] : [{ key, title, series: lines(found, names), axis: PERCENT_AXIS, format: formatPercent }];
}

function openChart(key: string, title: string, unit: 'bytesPerSecond' | 'load', drawn: readonly ChartSeries[]): Chart[] {
  const format = unit === 'load' ? formatLoad : formatBytesPerSecond;
  return drawn.length === 0 ? [] : [{ key, title, series: drawn, axis: { kind: 'open', unit }, format }];
}

function diskCharts(all: readonly MetricSeries[]): Chart[] {
  const mounts = ofKind(all, 'DISK_PERCENT').sort((a, b) => a.label.localeCompare(b.label));
  return mounts.flatMap((mount) => percentChart(`disk:${mount.label}`, `Disk ${mount.label}`, [mount], [mount.label]));
}

/** The first series of `kind` with the empty-label rule of WP-28 D-7 (cpu, memory, network, load carry a label we do not show). */
function first(all: readonly MetricSeries[], kind: MetricKind): MetricSeries[] {
  return ofKind(all, kind).slice(0, 1);
}

function networkChart(all: readonly MetricSeries[]): Chart[] {
  const found = [...first(all, 'NETWORK_IN_BPS'), ...first(all, 'NETWORK_OUT_BPS')];
  const names = found.map((series) => (series.kind === 'NETWORK_IN_BPS' ? 'In' : 'Out'));
  return openChart('network', 'Network', 'bytesPerSecond', lines(found, names));
}

function loadChart(all: readonly MetricSeries[]): Chart[] {
  const kinds = ['LOAD_1', 'LOAD_5', 'LOAD_15'] as const;
  const names = ['1 min', '5 min', '15 min'];
  const present = kinds.map((kind, at) => ({ series: first(all, kind)[0], name: names[at] ?? '' }));
  const found = present.flatMap(({ series }) => (series ? [series] : []));
  return openChart('load', 'Load', 'load', lines(found, present.flatMap(({ series, name }) => (series ? [name] : []))));
}

function gpuCharts(all: readonly MetricSeries[], gpus: readonly DeviceGpu[]): Chart[] {
  return [...gpus]
    .sort((a, b) => a.index - b.index)
    .flatMap((gpu) => {
      const title = `GPU ${String(gpu.index)} ${gpu.name}`;
      const ofGpu = (kind: MetricKind) => all.filter((series) => series.kind === kind && series.gpuIndex === gpu.index).slice(0, 1);
      const usage = percentChart(`gpu:${String(gpu.index)}:usage`, `${title} usage`, ofGpu('GPU_PERCENT'), ['Usage']);
      // MET-007: no VRAM chart for a GPU without its own memory, even if a series for it arrives.
      const vram = gpu.hasVram ? percentChart(`gpu:${String(gpu.index)}:vram`, `${title} VRAM`, ofGpu('VRAM_PERCENT'), ['VRAM']) : [];
      return [...usage, ...vram];
    });
}

/**
 * The charts the Overview draws, in a fixed order: CPU, memory, each disk mount (by mount point), network
 * (in then out), load (1, 5, 15), then per GPU by index its usage and its VRAM (MET-007, MET-009).
 * Uptime is a fact, not a chart. A chart is listed only when at least one of its series exists, so a device
 * that reports no load, or a GPU with no samples yet, shows no empty frame; a series that exists with no
 * points still gets its chart (the Overview shows its empty state).
 */
export function chartsOf(series: readonly MetricSeries[], gpus: readonly DeviceGpu[]): readonly Chart[] {
  return [
    ...percentChart('cpu', 'CPU', first(series, 'CPU_PERCENT'), ['CPU']),
    ...percentChart('memory', 'Memory', first(series, 'MEMORY_PERCENT'), ['Memory']),
    ...diskCharts(series),
    ...networkChart(series),
    ...loadChart(series),
    ...gpuCharts(series, gpus),
  ];
}
