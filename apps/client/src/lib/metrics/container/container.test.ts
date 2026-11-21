import { describe, expect, it } from 'vitest';
import { readings } from '../bars/bars.fixture';
import { containerRow, memoryBytesOfSeries, memoryLimit, mergeMemoryBytes, usedOfText } from './container';

const GIB = 1024 ** 3;
const MIB = 1024 ** 2;

describe('the figures of the container of a hosted hub (HMD-071, HMD-073)', () => {
  it('finds the limit from the bytes in use and their percent, and none without both', () => {
    expect(memoryLimit(0.5 * GIB, 25)).toBe(2 * GIB);
    expect(memoryLimit(null, 25)).toBeNull();
    expect(memoryLimit(GIB, null)).toBeNull();
    expect(memoryLimit(GIB, 0)).toBeNull();
    expect(memoryLimit(0.5 * GIB, 25.0004)).toBe(2 * GIB);
  });

  it('writes the memory as used of its limit in the unit of the limit', () => {
    expect(usedOfText(0.8 * GIB, 2 * GIB)).toBe('0.8 of 2.0 GiB');
    expect(usedOfText(1.25 * GIB, 4 * GIB)).toBe('1.3 of 4.0 GiB');
    expect(usedOfText(310 * MIB, 512 * MIB)).toBe('310 of 512 MiB');
  });

  it('takes the newest bytes in use from a series and from a batch, and keeps them through a batch with none', () => {
    const series = [{ kind: 'CPU_PERCENT', points: [{ value: 3 }] }, { kind: 'MEMORY_BYTES', points: [{ value: GIB }, { value: null }, { value: 2 * GIB }] }];
    expect(memoryBytesOfSeries(series)).toBe(2 * GIB);
    expect(memoryBytesOfSeries([])).toBeNull();
    expect(mergeMemoryBytes(GIB, [{ kind: 'MEMORY_BYTES', value: 3 * GIB }])).toBe(3 * GIB);
    expect(mergeMemoryBytes(GIB, [{ kind: 'CPU_PERCENT', value: 40 }])).toBe(GIB);
  });

  it('shows CPU and memory and no disk; the memory is its percent until the bytes in use are known', () => {
    const shown = readings({ cpuPercent: 91, memoryPercent: 40, diskPercent: null });
    const before = containerRow(shown, 'DAEMON', null);
    const after = containerRow(shown, 'DAEMON', 0.8 * GIB);
    if (before.kind !== 'bars' || after.kind !== 'bars') throw new Error('bars expected');
    expect(after.bars.map((bar) => bar.key)).toEqual(['cpu', 'memory']);
    expect(before.bars[1]?.text).toBe('40 %');
    expect(after.bars[1]).toMatchObject({ text: '0.8 of 2.0 GiB', percent: 40, spoken: '0.8 of 2.0 GiB, 40 %' });
    expect(after.bars[0]).toMatchObject({ text: '91 %', degraded: true });
  });

  it('shows the empty states of the other bars', () => {
    expect(containerRow(null, 'NONE', null)).toEqual({ kind: 'empty', text: 'No metrics' });
    expect(containerRow(null, 'DAEMON', null)).toEqual({ kind: 'empty', text: 'Waiting for metrics' });
  });
});
