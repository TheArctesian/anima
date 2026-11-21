import { expect, it } from 'vitest';
import { batteryBar, chargesOfSeries, mergeCharges } from './battery';

const charges = (entries: Record<string, number>) => new Map(Object.entries(entries));

it('MET-032: a device with no battery has no battery meter', () => {
  expect(batteryBar([], charges({ BAT0: 50 }))).toBeNull();
});

it('MET-032: a battery with no charge yet shows a dash, never 0, and its state in words', () => {
  const bar = batteryBar([{ name: 'BAT0', state: 'CHARGING' }], undefined);
  expect(bar?.text).toBe('—');
  expect(bar?.percent).toBeNull();
  expect(bar?.tooltip).toBe('Battery: —, charging');
});

it('MET-032: charging is a mark of its own and a note, not a colour', () => {
  const bar = batteryBar([{ name: 'BAT0', state: 'CHARGING' }], charges({ BAT0: 15 }));
  expect(bar).toMatchObject({ charging: true, low: false, degraded: false, note: 'charging', spoken: '15 %, charging' });
});

it('MET-032: the quiet warning at 20 % and, stronger, at 10 %, only while the battery discharges', () => {
  const at = (percent: number, state: 'DISCHARGING' | 'NOT_CHARGING' = 'DISCHARGING') => batteryBar([{ name: 'BAT0', state }], charges({ BAT0: percent }));
  expect(at(21)?.low).toBe(false);
  expect(at(20)).toMatchObject({ low: true, degraded: true, note: 'low' });
  expect(at(10)?.note).toBe('very low');
  expect(at(5, 'NOT_CHARGING')?.low).toBe(false);
});

it('MET-032: with several batteries the meter shows the lowest charge and names each battery in its words', () => {
  const bar = batteryBar(
    [
      { name: 'BAT0', state: 'CHARGING' },
      { name: 'BAT1', state: 'DISCHARGING' },
      { name: 'BAT2', state: 'UNKNOWN' },
    ],
    charges({ BAT0: 80, BAT1: 42 }),
  );
  expect(bar?.percent).toBe(42);
  expect(bar?.note).toBe('lowest of 3');
  expect(bar?.spoken).toBe('BAT1 42 %, on battery; BAT0 80 %, charging; BAT2 —, state unknown');
});

it('MET-032: a batch adds only the charges of kind BATTERY_PERCENT from 0 to 100, and keeps the others it knew', () => {
  const known = charges({ BAT0: 50, BAT1: 70 });
  const next = mergeCharges(known, [
    { kind: 'CPU_PERCENT', label: '', value: 12 },
    { kind: 'BATTERY_PERCENT', label: 'BAT0', value: 49 },
    { kind: 'BATTERY_PERCENT', label: 'BAT9', value: 140 },
  ]);
  expect(Object.fromEntries(next)).toEqual({ BAT0: 49, BAT1: 70 });
  expect(mergeCharges(known, [{ kind: 'CPU_PERCENT', label: '', value: 12 }])).toBe(known);
});

it('MET-032: the charge at load is the newest point of each battery series', () => {
  const series = [
    { kind: 'BATTERY_PERCENT', label: 'BAT0', points: [{ value: 60 }, { value: 58 }] },
    { kind: 'BATTERY_PERCENT', label: 'BAT1', points: [] },
    { kind: 'CPU_PERCENT', label: '', points: [{ value: 9 }] },
  ];
  expect(Object.fromEntries(chargesOfSeries(series))).toEqual({ BAT0: 58 });
});
