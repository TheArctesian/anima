import { expect, it } from 'vitest';
import type { CardReadingsFragment } from '$lib/hub/generated/graphql';
import { GPU, readings } from './bars.fixture';
import { percentText, barRow, otherDrivesText, type Bar, type BarReadings } from './bars';

function bars(over: Partial<CardReadingsFragment> = {}): readonly Bar[] {
  const row = barRow(readings(over), 'SSH');
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
  expect(barRow(null, 'NONE')).toEqual({ kind: 'empty', text: 'No metrics' });
  expect(barRow(readings(), 'NONE')).toEqual({ kind: 'empty', text: 'No metrics' });
  expect(barRow(null, 'AGENT')).toEqual({ kind: 'empty', text: 'Waiting for metrics' });
  expect(barRow(null, 'SSH')).toEqual({ kind: 'empty', text: 'Waiting for metrics' });
});

it('MET-007: a hub CardReadings fragment is assignable to BarReadings', () => {
  const fragment: CardReadingsFragment = readings({ vramPercent: 40, vramGpu: GPU });
  const input: BarReadings = fragment;
  expect(barRow(input, 'SSH').kind).toBe('bars');
});

it('MET-030: the disk bar is labelled with its mount and counts the other drives', () => {
  const disk = bars({ diskMount: '/', otherDrives: 3 })[2];
  expect(disk?.label).toBe('Disk /');
  expect(disk?.note).toBe('+3 drives');
  expect(disk?.tooltip).toBe('Disk /, 3 drives more on this device');
  expect(bars({ diskMount: '/mnt/archive', otherDrives: 0 })[2]).toMatchObject({ label: 'Disk /mnt/archive', note: null });
  expect(bars({ diskMount: null, otherDrives: 0 })[2]?.label).toBe('Disk');
});

it('MET-030: one other drive is singular, and none or a broken count is no note', () => {
  expect(otherDrivesText(1)).toBe('+1 drive');
  expect(otherDrivesText(0)).toBeNull();
  expect(otherDrivesText(-2)).toBeNull();
  expect(otherDrivesText(1.5)).toBeNull();
});
