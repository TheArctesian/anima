// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import Switch from './Switch.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(disabled = false): { flips: boolean[]; control: HTMLButtonElement } {
  const flips: boolean[] = [];
  app = mount(Switch, { target: document.body, props: { checked: false, label: 'SSH', disabled, onchange: (next: boolean) => flips.push(next) } });
  flushSync();
  const control = document.querySelector<HTMLButtonElement>('[role="switch"]');
  if (control === null) throw new Error('no switch');
  return { flips, control };
}

it('D17, NAV-005: a switch is named, reachable by Tab, says whether it is on, and tells each flip', () => {
  const { flips, control } = show();
  expect([control.getAttribute('aria-label'), control.getAttribute('tabindex'), control.getAttribute('aria-checked')]).toEqual(['SSH', '0', 'false']);
  control.click();
  flushSync();
  control.click();
  flushSync();
  expect(flips).toEqual([true, false]);
  expect(control.getAttribute('aria-checked')).toBe('false');
});

it('D17: a switch that cannot change keeps focus and its state, and tells nothing', () => {
  const { flips, control } = show(true);
  control.click();
  flushSync();
  expect(flips).toEqual([]);
  expect([control.getAttribute('aria-checked'), control.getAttribute('aria-disabled')]).toEqual(['false', 'true']);
});
