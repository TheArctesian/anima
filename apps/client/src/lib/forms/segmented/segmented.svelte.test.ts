// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import Segmented from './Segmented.svelte';
import { movedTo } from './segmented';

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const options = [
  { value: 'all', label: 'All', count: 12 },
  { value: 'up', label: 'Online', count: 9 },
  { value: 'down', label: 'Offline' },
];

function group(onchange: (value: string) => void = () => undefined) {
  app = mount(Segmented<string>, { target: document.body, props: { label: 'Status', options, value: 'all', onchange } });
  flushSync();
  return [...document.querySelectorAll<HTMLButtonElement>('[role="radio"]')];
}

async function press(from: HTMLElement, name: string): Promise<void> {
  from.focus();
  from.dispatchEvent(new KeyboardEvent('keydown', { key: name, bubbles: true, cancelable: true }));
  await Promise.resolve();
  flushSync();
}

it('UI-013: the group is a named radio group with one checked radio and one tab stop', () => {
  const radios = group();
  expect(document.querySelector('[role="radiogroup"]')?.getAttribute('aria-label')).toBe('Status');
  expect(radios.map((radio) => radio.getAttribute('aria-checked'))).toEqual(['true', 'false', 'false']);
  expect(radios.map((radio) => radio.tabIndex)).toEqual([0, -1, -1]);
});

it('UI-025: an option shows its count after the label, and one without a count shows none', () => {
  const radios = group();
  expect(radios.map((radio) => radio.querySelector('.count')?.textContent)).toEqual(['12', '9', undefined]);
});

it('UI-013: ArrowRight chooses and focuses the next option, moves the tab stop and reports the change', async () => {
  const onchange = vi.fn();
  const radios = group(onchange);
  await press(radios[0] as HTMLElement, 'ArrowRight');
  expect(onchange).toHaveBeenCalledExactlyOnceWith('up');
  expect(radios[1]?.getAttribute('aria-checked')).toBe('true');
  expect(document.activeElement).toBe(radios[1]);
  expect(radios.map((radio) => radio.tabIndex)).toEqual([-1, 0, -1]);
});

it('UI-013: arrow keys wrap, and Home and End go to the first and last option', async () => {
  const radios = group();
  await press(radios[0] as HTMLElement, 'ArrowLeft');
  expect(document.activeElement).toBe(radios[2]);
  await press(radios[2] as HTMLElement, 'Home');
  expect(document.activeElement).toBe(radios[0]);
  await press(radios[0] as HTMLElement, 'End');
  expect(document.activeElement).toBe(radios[2]);
});

it('UI-013: a click chooses; clicking the chosen option reports nothing', () => {
  const onchange = vi.fn();
  const radios = group(onchange);
  radios[0]?.click();
  expect(onchange).not.toHaveBeenCalled();
  radios[2]?.click();
  expect(onchange).toHaveBeenCalledExactlyOnceWith('down');
});

it('movedTo: other keys do not move', () => {
  expect(movedTo('a', 0, 3)).toBeUndefined();
  expect(movedTo('ArrowDown', 2, 3)).toBe(0);
});
