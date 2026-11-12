// @vitest-environment jsdom
import { mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import Table from './Table.svelte';
import type { Column } from './table';

interface Entry {
  readonly id: string;
  readonly message: string;
  readonly level: 'info' | 'warning' | 'error';
  readonly count: string;
  readonly day: string;
}

const COLUMNS: Column<Entry>[] = [
  { key: 'level', header: 'Severity', kind: 'severity', severity: (row) => row.level, min: 6 },
  { key: 'message', header: 'Message', value: (row) => row.message, min: 12, flex: true },
  { key: 'count', header: 'Count', kind: 'number', value: (row) => row.count, min: 5, max: 6 },
];

const LONG = 'The nightly backup of the media library on the network attached storage finished with 4 warnings';
const ROWS: Entry[] = [
  { id: 'a', message: LONG, level: 'warning', count: '12', day: 'Today' },
  { id: 'b', message: 'Check passed', level: 'info', count: '3', day: 'Yesterday' },
];

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(extra: Record<string, unknown> = {}): void {
  app = mount(Table<Entry>, { target: document.body, props: { columns: COLUMNS, rows: ROWS, rowKey: (row: Entry) => row.id, caption: 'Events', ...extra } });
}

it('UI-027: a long text cell is cut by CSS and its full text is its title', () => {
  show();
  const cell = document.querySelector<HTMLElement>('td.text');
  expect(cell?.getAttribute('title')).toBe(LONG);
  expect(cell?.textContent).toBe(LONG);
});

it('UI-003: a severity cell shows the word and a dot, and a number cell is marked for tabular right alignment', () => {
  show();
  expect([...document.querySelectorAll('.severity')].map((cell) => cell.textContent.trim())).toEqual(['Warning', 'Info']);
  expect([...document.querySelectorAll('td.number')].map((cell) => cell.textContent)).toEqual(['12', '3']);
  expect(document.querySelector('th.number')?.textContent).toBe('Count');
});

it('Accessibility: a real table named by a caption, with column header cells scoped to their column', () => {
  show();
  expect(document.querySelector('table > caption')?.textContent).toBe('Events');
  const headers = [...document.querySelectorAll('thead th')];
  expect(headers.map((header) => header.getAttribute('scope'))).toEqual(['col', 'col', 'col']);
  expect(headers.map((header) => header.textContent)).toEqual(['Severity', 'Message', 'Count']);
});

it('UI-025: grouped rows get a rowgroup header spanning every column', () => {
  show({ groupOf: (row: Entry) => row.day });
  const groups = [...document.querySelectorAll('tr.group th')];
  expect(groups.map((group) => group.textContent)).toEqual(['Today', 'Yesterday']);
  expect(groups[0]?.getAttribute('scope')).toBe('rowgroup');
  expect(groups[0]?.getAttribute('colspan')).toBe('3');
});

it('UI-013: density is a class on the table; a flexible column has no width, the others do', () => {
  show({ density: 'dense' });
  expect(document.querySelector('table')?.classList.contains('dense')).toBe(true);
  expect([...document.querySelectorAll<HTMLElement>('col')].map((col) => col.style.width)).toEqual(['6rem', '', '6rem']);
});
