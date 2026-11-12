import { expect, it } from 'vitest';
import { columnWidth, dayLabel, tableMinWidth, withGroupHeaders, type Column } from './table';

const NOW = new Date(2026, 10, 11, 9, 30);

it('UI-025: a group header reads Today, Yesterday, then the date', () => {
  expect(dayLabel(new Date(2026, 10, 11, 0, 1), NOW)).toBe('Today');
  expect(dayLabel(new Date(2026, 10, 10, 23, 59), NOW)).toBe('Yesterday');
  expect(dayLabel(new Date(2026, 10, 9, 12, 0), NOW)).toBe('9 Nov 2026');
});

it('UI-025: a header row opens each run of rows that share a group, and there is none without grouping', () => {
  const rows = [{ day: 'Today' }, { day: 'Today' }, { day: 'Yesterday' }];
  const entries = withGroupHeaders(rows, (row) => row.day);
  expect(entries.map((entry) => ('group' in entry ? `#${entry.group}` : entry.row.day))).toEqual(['#Today', 'Today', 'Today', '#Yesterday', 'Yesterday']);
  expect(withGroupHeaders(rows)).toHaveLength(3);
});

it('DEV-007: a flexible column takes what is left, others their max or min, and the table keeps the sum of the minimums', () => {
  const columns: Column<unknown>[] = [
    { key: 'name', header: 'Name', min: 10, flex: true },
    { key: 'uptime', header: 'Uptime', kind: 'number', min: 5, max: 6 },
    { key: 'at', header: 'At', min: 8 },
  ];
  expect(columns.map((column) => columnWidth(column))).toEqual([undefined, '6rem', '8rem']);
  expect(tableMinWidth(columns)).toBe('23rem');
});
