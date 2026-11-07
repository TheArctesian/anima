import type { MetricsSource, Readings } from '$lib/devices/api';

/** MET-004: a metric strictly above this is degraded. */
export const DEGRADED_ABOVE = 85;

export interface Bar {
  readonly key: 'cpu' | 'memory' | 'disk' | 'vram';
  readonly label: string;
  /** The reading, 0 to 100; `null` while it is unknown. */
  readonly percent: number | null;
  /** The whole-percent text, or "—" for an unknown reading. */
  readonly text: string;
  readonly degraded: boolean;
  /** Names the GPU for the VRAM bar. */
  readonly tooltip: string;
}

export type ReadingsRow =
  | { readonly kind: 'bars'; readonly bars: readonly Bar[] }
  | { readonly kind: 'empty'; readonly text: string };

const UNKNOWN = '—';

/** MET-004: whole percent; a missing value is a dash, never 0 (MET-007). */
export function percentText(percent: number | null): string {
  return percent === null ? UNKNOWN : `${String(Math.round(percent))} %`;
}

function bar(key: Bar['key'], label: string, percent: number | null, tooltip = label): Bar {
  return { key, label, percent, text: percentText(percent), degraded: percent !== null && percent > DEGRADED_ABOVE, tooltip };
}

/** CPU, Memory, Disk, then VRAM only for a GPU with its own memory (MET-007, MET-008). */
function barsOf(readings: Readings): readonly Bar[] {
  const bars = [
    bar('cpu', 'CPU', readings.cpuPercent),
    bar('memory', 'Memory', readings.memoryPercent),
    bar('disk', 'Disk', readings.diskPercent),
  ];
  const gpu = readings.vramGpu;
  if (gpu?.hasVram === true && readings.vramPercent !== null) {
    bars.push(bar('vram', 'VRAM', readings.vramPercent, `VRAM, GPU ${String(gpu.index)}: ${gpu.name}`));
  }
  return bars;
}

/** What the card's metric row shows for a device's readings and metrics source (6.1, MET-007). */
export function readingsRow(readings: Readings | null, source: MetricsSource | null | undefined): ReadingsRow {
  if (source !== 'SSH') return { kind: 'empty', text: 'No metrics' };
  if (readings === null) return { kind: 'empty', text: 'Waiting for metrics' };
  return { kind: 'bars', bars: barsOf(readings) };
}
