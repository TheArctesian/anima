import type { MetricSeries, NewMetrics } from './api';
import { rangeWindow } from './ranges';

/** A GPU series is labelled with its index as decimal text (R-099). */
const GPU_LABEL = /^(0|[1-9]\d*)$/;

function gpuIndexOf(kind: string, label: string): number | null {
  const index = kind.startsWith('GPU_') || kind.startsWith('VRAM_') ? GPU_LABEL.exec(label)?.[1] : undefined;
  return index === undefined ? null : Number(index);
}

/** A GPU series is told apart by kind and GPU index, as its chart label is `GPU <index> <name>`; any other by kind and label. */
function seriesKey(kind: string, label: string, gpuIndex: number | null): string {
  return gpuIndex === null ? `${kind}\u0000${label}` : `${kind}\u0000gpu${String(gpuIndex)}`;
}

/**
 * Add a live batch to the HOUR series (D-5). Each sample becomes a point at `added.at` on the series with the
 * same kind and label, or for a GPU the same kind and GPU index; a sample for an unknown series starts a new one. A sample not newer than the series'
 * last point is ignored; a batch with an unreadable time changes nothing, and a point with one, is dropped rather than trusted. Points older than `added.at` minus one hour drop off the left. The input is not changed.
 */
export function mergeLive(series: readonly MetricSeries[], added: NewMetrics): readonly MetricSeries[] {
  const at = Date.parse(added.at);
  if (Number.isNaN(at)) return series;
  const oldest = rangeWindow('HOUR', new Date(at)).start;
  const merged = new Map<string, MetricSeries>(series.map((each) => [seriesKey(each.kind, each.label, each.gpuIndex), each]));
  for (const sample of added.samples) {
    const gpuIndex = gpuIndexOf(sample.kind, sample.label);
    const id = seriesKey(sample.kind, sample.label, gpuIndex);
    const current = merged.get(id) ?? {
      kind: sample.kind,
      label: gpuIndex === null ? sample.label : `GPU ${String(gpuIndex)}`,
      gpuIndex,
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
