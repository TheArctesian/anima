import type { MetricRange, MetricSeries, NewMetrics } from './api';

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;
const SPAN_MS: Record<MetricRange, number> = { HOUR: HOUR_MS, DAY: DAY_MS, WEEK: 7 * DAY_MS };
const GPU_LABEL = /^GPU (\d+) /;

/** The time window a range shows at `now`: HOUR 1 h, DAY 24 h, WEEK 7 d (MET-003). */
export function rangeWindow(range: MetricRange, now: Date): { readonly start: Date; readonly end: Date } {
  return { start: new Date(now.getTime() - SPAN_MS[range]), end: now };
}

function gpuIndexOf(kind: string, label: string): number | null {
  const index = kind.startsWith('GPU_') || kind.startsWith('VRAM_') ? GPU_LABEL.exec(label)?.[1] : undefined;
  return index === undefined ? null : Number(index);
}

/**
 * Add a live batch to the HOUR series (D-5). Each sample becomes a point at `added.at` on the series with the
 * same kind and label; a sample for an unknown series starts a new one. A sample not newer than the series'
 * last point is ignored. Points older than `added.at` minus one hour drop off the left. The input is not changed.
 */
export function mergeLive(series: readonly MetricSeries[], added: NewMetrics): readonly MetricSeries[] {
  const at = Date.parse(added.at);
  const oldest = at - HOUR_MS;
  const merged = new Map<string, MetricSeries>(series.map((each) => [`${each.kind}\u0000${each.label}`, each]));
  for (const sample of added.samples) {
    const id = `${sample.kind}\u0000${sample.label}`;
    const current = merged.get(id) ?? {
      kind: sample.kind,
      label: sample.label,
      gpuIndex: gpuIndexOf(sample.kind, sample.label),
      points: [],
    };
    const last = current.points.at(-1);
    if (last !== undefined && Date.parse(last.at) >= at) continue;
    const point = { at: added.at, value: sample.value, min: null, max: null };
    merged.set(id, { ...current, points: [...current.points, point] });
  }
  return [...merged.values()].map((each) => ({
    ...each,
    points: each.points.filter((point) => Date.parse(point.at) >= oldest),
  }));
}
