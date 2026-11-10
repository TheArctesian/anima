import { describe, expect, it } from 'vitest';
import { mergeLive } from './live';
import { added, CPU_MINUTES, point, series } from './metrics.fixture';

const CPU = series('CPU_PERCENT', 'cpu', CPU_MINUTES);

describe('mergeLive (D-5)', () => {
  it('appends a sample as a point at its time', () => {
    const next = mergeLive([CPU], added('2026-11-05T09:05:00Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 20 }]));
    expect(next).toHaveLength(1);
    expect(next[0]?.points.at(-1)).toMatchObject({ at: '2026-11-05T09:05:00Z', value: 20 });
    expect(next[0]?.points).toHaveLength(6);
  });

  it('ignores a sample not newer than the last point', () => {
    const same = mergeLive([CPU], added('2026-11-05T09:04:00Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 99 }]));
    const older = mergeLive([CPU], added('2026-11-05T09:02:30Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 99 }]));
    expect(same[0]?.points).toEqual(CPU_MINUTES);
    expect(older[0]?.points).toEqual(CPU_MINUTES);
  });

  it('starts a new series for an unknown kind and label, with a GPU index only for GPU series', () => {
    const next = mergeLive(
      [CPU],
      added('2026-11-05T09:05:00Z', [
        { kind: 'DISK_PERCENT', label: '/data', value: 30 },
        { kind: 'GPU_PERCENT', label: '1', value: 8 },
      ]),
    );
    expect(next.map((each) => [each.kind, each.label, each.gpuIndex])).toEqual([
      ['CPU_PERCENT', 'cpu', null],
      ['DISK_PERCENT', '/data', null],
      ['GPU_PERCENT', 'GPU 1', 1],
    ]);
  });

  it('keeps series of one kind apart by label', () => {
    const disks = [series('DISK_PERCENT', '/', [point('2026-11-05T09:00:00Z', 60)])];
    const next = mergeLive(disks, added('2026-11-05T09:01:00Z', [{ kind: 'DISK_PERCENT', label: '/data', value: 30 }]));
    expect(next).toHaveLength(2);
    expect(next[0]?.points).toHaveLength(1);
  });

  it('drops points older than one hour before the new point', () => {
    const old = series('CPU_PERCENT', 'cpu', [
      point('2026-11-05T09:00:00Z', 1),
      point('2026-11-05T09:30:00Z', 2),
    ]);
    const next = mergeLive([old], added('2026-11-05T10:00:30Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 3 }]));
    expect(next[0]?.points.map((each) => each.value)).toEqual([2, 3]);
  });

  it('returns the input unchanged for an unreadable batch time', () => {
    const input = [CPU];
    expect(mergeLive(input, added('not a time', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 1 }]))).toBe(input);
  });

  it('does not wipe a series whose last point has an unreadable time', () => {
    const odd = series('CPU_PERCENT', 'cpu', [point('garbage', 1)]);
    const next = mergeLive([odd], added('2026-11-05T09:05:00Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 2 }]));
    expect(next[0]?.points.map((each) => each.value)).toEqual([2]);
  });

  it('does not change its input', () => {
    mergeLive([CPU], added('2026-11-05T09:05:00Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 20 }]));
    expect(CPU.points).toHaveLength(5);
  });
});

describe('mergeLive GPU series (MET-009, R-099)', () => {
  const gpuIndexes = (samples: { kind: 'GPU_PERCENT' | 'VRAM_PERCENT' | 'DISK_PERCENT'; label: string }[]) =>
    mergeLive([], added('2026-11-05T09:05:00Z', samples.map((sample) => ({ ...sample, value: 1 })))).map(
      (each) => each.gpuIndex,
    );

  it('joins a live GPU sample to the chart series of that GPU, whatever its label', () => {
    const chart = series('GPU_PERCENT', 'GPU 0 RTX 4090', [point('2026-11-05T09:00:00Z', 10)], 0);
    const next = mergeLive([chart], added('2026-11-05T09:05:00Z', [{ kind: 'GPU_PERCENT', label: '0', value: 50 }]));
    expect(next.map((each) => [each.label, each.gpuIndex, each.points.length])).toEqual([['GPU 0 RTX 4090', 0, 2]]);
  });

  it('starts a series labelled GPU <index> for a live GPU sample with no chart series yet', () => {
    const next = mergeLive([], added('2026-11-05T09:05:00Z', [{ kind: 'VRAM_PERCENT', label: '2', value: 50 }]));
    expect(next.map((each) => [each.kind, each.label, each.gpuIndex])).toEqual([['VRAM_PERCENT', 'GPU 2', 2]]);
  });

  it('reads the GPU index from a GPU or VRAM label that is decimal text', () => {
    expect(
      gpuIndexes([
        { kind: 'GPU_PERCENT', label: '3' },
        { kind: 'VRAM_PERCENT', label: '12' },
      ]),
    ).toEqual([3, 12]);
  });

  it('gives no GPU index to a label that is not decimal text, or to a kind that is not a GPU one', () => {
    expect(
      gpuIndexes([
        { kind: 'GPU_PERCENT', label: 'GPU 1 Intel' },
        { kind: 'VRAM_PERCENT', label: '01' },
        { kind: 'DISK_PERCENT', label: '7' },
      ]),
    ).toEqual([null, null, null]);
  });
});
