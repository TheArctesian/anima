import type { TimeWindow } from '$lib/charts/scales';
import type { MetricRange, MetricSeries, NewMetrics } from './api';

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;
const SPAN_MS: Record<MetricRange, number> = { HOUR: HOUR_MS, DAY: DAY_MS, WEEK: 7 * DAY_MS };
const GPU_LABEL = /^GPU (\d+) /;

/** The time window a range shows at `now`, in milliseconds: HOUR 1 h, DAY 24 h, WEEK 7 d (MET-003). */
export function rangeWindow(range: MetricRange, now: Date): TimeWindow {
  return { start: now.getTime() - SPAN_MS[range], end: now.getTime() };
}

/** WP-28 D-5: the width of one bucket of each range, in milliseconds. */
const BUCKET_MS: Record<MetricRange, number> = { HOUR: 60_000, DAY: 600_000, WEEK: HOUR_MS };

/** A wider gap than this between two points breaks the line (MET-003): two and a half buckets, so a late point does not break it and two missing buckets do. */
export function rangeGap(range: MetricRange): number {
  return BUCKET_MS[range] * 2.5;
}

function gpuIndexOf(kind: string, label: string): number | null {
  const index = kind.startsWith('GPU_') || kind.startsWith('VRAM_') ? GPU_LABEL.exec(label)?.[1] : undefined;
  return index === undefined ? null : Number(index);
}

/**
 * Add a live batch to the HOUR series (D-5). Each sample becomes a point at `added.at` on the series with the
 * same kind and label; a sample for an unknown series starts a new one. A sample not newer than the series'
 * last point is ignored; a batch with an unreadable time changes nothing, and a point with one, is dropped rather than trusted. Points older than `added.at` minus one hour drop off the left. The input is not changed.
 */
export function mergeLive(series: readonly MetricSeries[], added: NewMetrics): readonly MetricSeries[] {
  const at = Date.parse(added.at);
  if (Number.isNaN(at)) return series;
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
    const last = current.points.findLast((each) => Number.isFinite(Date.parse(each.at)));
    if (last !== undefined && Date.parse(last.at) >= at) continue;
    const point = { at: added.at, value: sample.value, min: null, max: null };
    merged.set(id, { ...current, points: [...current.points, point] });
  }
  return [...merged.values()].map((each) => ({
    ...each,
    points: each.points.filter((point) => Date.parse(point.at) >= oldest),
  }));
}
