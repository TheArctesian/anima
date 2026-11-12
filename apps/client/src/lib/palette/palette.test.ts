// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import PaletteRow from './PaletteRow.svelte';
import { applyPalette, DEFAULT_PALETTE, PALETTES, storedPalette, storePalette } from './palette';

let app: ReturnType<typeof mount> | undefined;

function show(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(PaletteRow, { target });
  flushSync();
  return target;
}

function colours(target: HTMLElement): HTMLSelectElement {
  const control = [...target.querySelectorAll('label')].find((each) => each.textContent === 'Colours')?.control;
  if (!(control instanceof HTMLSelectElement)) throw new Error('no Colours');
  return control;
}

function choose(control: HTMLSelectElement, value: string): void {
  control.value = value;
  control.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
  delete document.documentElement.dataset['palette'];
  vi.restoreAllMocks();
});

it('UI-032: the palette list starts with Graphite, the default, then Nord', () => {
  expect(DEFAULT_PALETTE).toBe('graphite');
  expect(PALETTES.map((palette) => palette.id)).toEqual(['graphite', 'nord']);
});

it('UI-032: an unknown stored value, and a storage that refuses to be read, give Graphite', () => {
  localStorage.setItem('anima.palette', 'solarized');
  expect(storedPalette(localStorage)).toBe('graphite');
  const refusing = { ...localStorage, getItem: () => { throw new Error('refused'); } } as unknown as Storage;
  expect(storedPalette(refusing)).toBe('graphite');
});

it('UI-032: a saved palette is read back from anima.palette, and applyPalette sets data-palette', () => {
  storePalette(localStorage, 'nord');
  expect(localStorage.getItem('anima.palette')).toBe('nord');
  expect(storedPalette(localStorage)).toBe('nord');
  const root = document.createElement('html');
  applyPalette(root, 'nord');
  expect(root.dataset['palette']).toBe('nord');
});

it('UI-032: the labelled Colours choice offers Graphite and Nord, and each choice applies at once and lasts', async () => {
  const control = colours(show());
  expect([...control.options].map((option) => option.textContent)).toEqual(['Graphite', 'Nord']);
  expect(control.value).toBe('graphite');
  choose(control, 'nord');
  expect(document.documentElement.dataset['palette']).toBe('nord');
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  expect(colours(show()).value).toBe('nord');
});

it('UI-032: a storage that refuses to save still applies the palette and says it will not last', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('quota');
  });
  const target = show();
  choose(colours(target), 'nord');
  expect(document.documentElement.dataset['palette']).toBe('nord');
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(
    'Anima could not save this choice on this device; it lasts until the app restarts.',
  );
});
