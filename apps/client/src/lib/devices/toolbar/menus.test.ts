// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { DEFAULT_QUERY, type ListQuery } from '../query';
import Toolbar from './Toolbar.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;
const changes: ListQuery[] = [];
const added: string[] = [];

function show(query: ListQuery = DEFAULT_QUERY): void {
  changes.length = 0;
  app = mount(Toolbar, {
    target: document.body,
    props: {
      query,
      tags: ['lab'],
      tagsNote: 'No tags yet',
      onQueryChange: (next: ListQuery) => changes.push(next),
      onAddDevice: () => added.push('add'),
    },
  });
  flushSync();
}

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

const control = (label: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('.control')].find((node) => node.textContent.includes(label)) as HTMLButtonElement;
const panel = () => document.querySelector('.panel');

it('Menu: the control opens its panel and says so with aria-expanded', () => {
  show();
  expect(control('Status').getAttribute('aria-expanded')).toBe('false');
  control('Status').click();
  flushSync();
  expect(control('Status').getAttribute('aria-expanded')).toBe('true');
  expect(panel()?.querySelectorAll('input[type=checkbox]')).toHaveLength(5);
});

it('Menu: Escape closes the panel and returns focus to the control', async () => {
  show();
  control('Tags').click();
  flushSync();
  panel()?.querySelector('input')?.focus();
  panel()?.querySelector('input')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
  expect(document.activeElement).toBe(control('Tags'));
});

it('Menu: a press outside closes the panel and a press inside does not', async () => {
  show();
  control('Sort').click();
  flushSync();
  panel()?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  expect(panel()).not.toBeNull();
  document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
});

it('Menu: focus inside the panel returns to the control on an outside press', async () => {
  show();
  control('Status').click();
  flushSync();
  panel()?.querySelector('input')?.focus();
  document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  expect(document.activeElement).toBe(control('Status'));
});

it('DEV-003: the Status menu counts the chosen statuses in its label', () => {
  show({ ...DEFAULT_QUERY, statuses: ['UP', 'DOWN'] });
  expect(control('Status').textContent).toContain('Status (2)');
});

it('DEV-004: Sort is one choice among Name, Status and Uptime', () => {
  show({ ...DEFAULT_QUERY, sort: 'STATUS' });
  control('Sort').click();
  flushSync();
  const radios = [...(panel()?.querySelectorAll<HTMLInputElement>('input[type=radio]') ?? [])];
  expect(radios.map((radio) => radio.checked)).toEqual([false, true, false]);
});

it('D-4: the view toggle shows the current view with aria-pressed and writes the other', () => {
  show();
  const [grid, table] = [...document.querySelectorAll<HTMLButtonElement>('.views button')];
  expect(grid?.getAttribute('aria-pressed')).toBe('true');
  expect(table?.getAttribute('aria-pressed')).toBe('false');
  table?.click();
  expect(changes.at(-1)?.view).toBe('table');
});

it('H-1: the Add device button calls onAddDevice', () => {
  show();
  [...document.querySelectorAll('button')].find((node) => node.textContent.includes('Add device'))?.click();
  expect(added).toEqual(['add']);
});

it('DEV-003: an empty tag menu says why', () => {
  show();
  control('Tags').click();
  flushSync();
  expect(panel()?.querySelectorAll('input')).toHaveLength(1);
});

it('D-4: unmounting with a keystroke pending writes nothing', async () => {
  vi.useFakeTimers();
  show();
  const input = document.querySelector<HTMLInputElement>('input[type=search]');
  if (input === null) throw new Error('no search');
  input.value = 'abc';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await unmount(app as object);
  app = undefined;
  vi.advanceTimersByTime(1000);
  vi.useRealTimers();
  expect(changes).toEqual([]);
});

it('Menu: a press outside does not steal focus that is somewhere else', () => {
  show();
  control('Status').click();
  flushSync();
  const input = document.querySelector<HTMLInputElement>('input[type=search]');
  input?.focus();
  document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  expect(document.activeElement).toBe(input);
});

it('Menu: Escape closes a menu a mouse click opened, with focus outside it (WebKit does not focus the button)', async () => {
  show();
  control('Sort').click();
  flushSync();
  (document.activeElement as HTMLElement | null)?.blur();
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
  expect(document.activeElement).toBe(control('Sort'));
});

it('WebKit: every button and option has tabindex 0, or Tab skips it (NAV-005)', () => {
  show();
  control('Status').click();
  flushSync();
  const controls = [...document.querySelectorAll('button, a[href], input[type=checkbox], input[type=radio]')];
  expect(controls.length).toBeGreaterThan(8);
  expect(controls.filter((node) => node.getAttribute('tabindex') !== '0')).toEqual([]);
});
