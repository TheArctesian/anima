import { describe, expect, it } from 'vitest';
import { mergeLive, rangeWindow } from './live';
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

  it('starts a new series for an unknown kind and label, with the GPU index from the label', () => {
    const next = mergeLive(
      [CPU],
      added('2026-11-05T09:05:00Z', [
        { kind: 'DISK_PERCENT', label: '/data', value: 30 },
        { kind: 'GPU_PERCENT', label: 'GPU 1 Intel UHD Graphics', value: 8 },
      ]),
    );
    expect(next.map((each) => [each.kind, each.label, each.gpuIndex])).toEqual([
      ['CPU_PERCENT', 'cpu', null],
      ['DISK_PERCENT', '/data', null],
      ['GPU_PERCENT', 'GPU 1 Intel UHD Graphics', 1],
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

describe('rangeWindow (MET-003)', () => {
  const now = new Date('2026-11-05T12:00:00Z');
  it('spans 1 hour, 24 hours and 7 days ending at now', () => {
    const start = (range: 'HOUR' | 'DAY' | 'WEEK') => new Date(rangeWindow(range, now).start).toISOString();
    expect(start('HOUR')).toBe('2026-11-05T11:00:00.000Z');
    expect(start('DAY')).toBe('2026-11-04T12:00:00.000Z');
    expect(start('WEEK')).toBe('2026-10-29T12:00:00.000Z');
    expect(rangeWindow('WEEK', now).end).toBe(now.getTime());
  });
});
