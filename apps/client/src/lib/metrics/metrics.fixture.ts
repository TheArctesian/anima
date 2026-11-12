import type { DeviceGpu, MetricSeries, NewMetrics } from './api';

type MetricPoint = MetricSeries['points'][number];

/** Synthetic values after the K6 `phase2.json` metrics (CPU HOUR minutes, WEEK GPU values); no real device. */
export const NVIDIA: DeviceGpu = { index: 0, name: 'NVIDIA GeForce RTX 3060', vendor: 'NVIDIA', hasVram: true };
export const INTEGRATED: DeviceGpu = { index: 1, name: 'Intel UHD Graphics', vendor: 'INTEL', hasVram: false };

/** An Apple silicon GPU: its memory is the Mac's own, so it has no VRAM (MET-007, D-30). */
export const APPLE: DeviceGpu = { index: 0, name: 'Apple M2 Pro', vendor: 'APPLE', hasVram: false };

export function point(at: string, value: number): MetricPoint {
  return { at, value, min: value - 3, max: value + 3 };
}

export function series(
  kind: MetricSeries['kind'],
  label: string,
  points: MetricPoint[] = [],
  gpuIndex: number | null = null,
): MetricSeries {
  return { kind, label, gpuIndex, points };
}

export const CPU_MINUTES = [
  point('2026-11-05T09:00:00Z', 14.2),
  point('2026-11-05T09:01:00Z', 15.8),
  point('2026-11-05T09:02:00Z', 13.1),
  point('2026-11-05T09:03:00Z', 22.6),
  point('2026-11-05T09:04:00Z', 18.4),
];

export const CPU_ONLY: readonly MetricSeries[] = [
  series('CPU_PERCENT', 'cpu', CPU_MINUTES),
  series('MEMORY_PERCENT', 'memory', [point('2026-11-05T09:04:00Z', 45)]),
];

export const TWO_GPUS: readonly MetricSeries[] = [
  ...CPU_ONLY,
  series('DISK_PERCENT', '/', [point('2026-11-05T09:04:00Z', 61)]),
  series('DISK_PERCENT', '/data', [point('2026-11-05T09:04:00Z', 30)]),
  series('NETWORK_IN_BPS', 'eth0', [point('2026-11-05T09:04:00Z', 1_500_000)]),
  series('NETWORK_OUT_BPS', 'eth0', [point('2026-11-05T09:04:00Z', 400_000)]),
  series('LOAD_1', '', [point('2026-11-05T09:04:00Z', 0.4)]),
  series('LOAD_5', '', [point('2026-11-05T09:04:00Z', 0.5)]),
  series('LOAD_15', '', [point('2026-11-05T09:04:00Z', 0.6)]),
  series('UPTIME_SECONDS', '', [point('2026-11-05T09:04:00Z', 90_000)]),
  series('GPU_PERCENT', `GPU 0 ${NVIDIA.name}`, [point('2026-11-05T09:04:00Z', 35.2)], 0),
  series('VRAM_PERCENT', `GPU 0 ${NVIDIA.name}`, [point('2026-11-05T09:04:00Z', 20)], 0),
  series('GPU_PERCENT', `GPU 1 ${INTEGRATED.name}`, [point('2026-11-05T09:04:00Z', 8)], 1),
  series('VRAM_PERCENT', `GPU 1 ${INTEGRATED.name}`, [point('2026-11-05T09:04:00Z', 12)], 1),
];

export function added(at: string, samples: NewMetrics['samples']): NewMetrics {
  return {
    deviceId: '0190a000-0000-7000-8012-000000000004',
    at,
    samples,
    readings: { at, cpuPercent: null, memoryPercent: null, diskPercent: null, diskMount: null, otherDrives: 0, vramPercent: null, vramGpu: null },
  };
}
