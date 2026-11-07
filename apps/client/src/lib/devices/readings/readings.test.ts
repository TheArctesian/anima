import { expect, it } from 'vitest';
import type { CardReadingsFragment } from '$lib/hub/generated/graphql';
import { percentText, readingsRow, type Bar } from './readings';

export const GPU = { index: 1, name: 'RTX 4090', vendor: 'NVIDIA', hasVram: true } as const;

export function readings(over: Partial<CardReadingsFragment> = {}): CardReadingsFragment {
  return { at: '2026-01-01T00:00:00Z', cpuPercent: 10, memoryPercent: 20, diskPercent: 30, vramPercent: null, vramGpu: null, ...over };
}

function bars(over: Partial<CardReadingsFragment> = {}): readonly Bar[] {
  const row = readingsRow(readings(over), 'SSH');
  if (row.kind !== 'bars') throw new Error('expected bars');
  return row.bars;
}

it('MET-007: no VRAM bar without a GPU that has its own memory, and never "0 %"', () => {
  expect(bars().map((bar) => bar.key)).toEqual(['cpu', 'memory', 'disk']);
  expect(bars({ vramPercent: 0, vramGpu: { ...GPU, hasVram: false } })).toHaveLength(3);
  expect(bars({ vramPercent: null, vramGpu: GPU })).toHaveLength(3);
});

it('MET-008: VRAM comes after disk and names the GPU with the highest use', () => {
  const list = bars({ vramPercent: 40, vramGpu: GPU });
  expect(list.map((bar) => bar.key)).toEqual(['cpu', 'memory', 'disk', 'vram']);
  expect(list[3]?.tooltip).toBe('VRAM, GPU 1: RTX 4090');
});

it('MET-004: 85 is not degraded and 85.1 is', () => {
  const [cpu, memory] = bars({ cpuPercent: 85, memoryPercent: 85.1 });
  expect(cpu?.degraded).toBe(false);
  expect(memory?.degraded).toBe(true);
});

it('MET-004: the value text is a whole percent', () => {
  expect(percentText(42.4)).toBe('42 %');
  expect(percentText(42.5)).toBe('43 %');
});

it('MET-007: a null value shows a dash, not 0, and is not degraded', () => {
  const [cpu] = bars({ cpuPercent: null });
  expect(cpu?.text).toBe('—');
  expect(cpu?.degraded).toBe(false);
});

it('6.1: source NONE says "No metrics", SSH without readings says "Waiting for metrics"', () => {
  expect(readingsRow(null, 'NONE')).toEqual({ kind: 'empty', text: 'No metrics' });
  expect(readingsRow(readings(), 'NONE')).toEqual({ kind: 'empty', text: 'No metrics' });
  expect(readingsRow(null, 'SSH')).toEqual({ kind: 'empty', text: 'Waiting for metrics' });
});
