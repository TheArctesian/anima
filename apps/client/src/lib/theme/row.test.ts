// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import ThemeRow from './ThemeRow.svelte';

let app: ReturnType<typeof mount> | undefined;

function show(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(ThemeRow, { target });
  flushSync();
  return target;
}

function themeSelect(target: HTMLElement): HTMLSelectElement {
  const found = [...target.querySelectorAll('label')].find((each) => each.querySelector('span')?.textContent === 'Theme');
  const control = found?.querySelector('select');
  if (control == null) throw new Error('no Theme');
  return control;
}

function choose(control: HTMLSelectElement, value: string): void {
  control.value = value;
  control.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

async function restart(): Promise<HTMLElement> {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  return show();
}

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
  delete document.documentElement.dataset['theme'];
  Reflect.deleteProperty(document, 'startViewTransition');
  vi.restoreAllMocks();
});

it('UI-005: the labelled Theme choice offers Dark, Light and System, and starts at Dark', () => {
  const control = themeSelect(show());
  expect([...control.options].map((option) => option.textContent)).toEqual(['Dark', 'Light', 'System']);
  expect(control.value).toBe('dark');
});

it('UI-005: each choice sets data-theme on <html> at once and is still chosen after a restart', async () => {
  for (const theme of ['light', 'system', 'dark']) {
    choose(themeSelect(await restart()), theme);
    expect(document.documentElement.dataset['theme']).toBe(theme);
    expect(localStorage.getItem('anima.theme')).toBe(theme);
    expect(themeSelect(await restart()).value).toBe(theme);
  }
});

it('UI-005: an unknown stored value shows Dark', () => {
  localStorage.setItem('anima.theme', 'sepia');
  expect(themeSelect(show()).value).toBe('dark');
});

it('UI-005: a storage that refuses to save still applies the theme and says it will not last', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('quota');
  });
  const target = show();
  choose(themeSelect(target), 'light');
  expect(document.documentElement.dataset['theme']).toBe('light');
  expect(themeSelect(target).value).toBe('light');
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(
    'Anima could not save this choice on this device; it lasts until the app restarts.',
  );
});

it('UI-005: the "could not save" message goes once a later choice is saved', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementationOnce(() => {
    throw new Error('quota');
  });
  const target = show();
  choose(themeSelect(target), 'light');
  expect(target.querySelector('[role="alert"]')).not.toBeNull();
  choose(themeSelect(target), 'system');
  expect(target.querySelector('[role="alert"]')).toBeNull();
  expect(localStorage.getItem('anima.theme')).toBe('system');
});

/** A browser with view transitions: records each start and keeps the update for the test to run as the next frame. */
function viewTransitions(): { starts: number; frame: () => void } {
  const pending: (() => void)[] = [];
  const record = { starts: 0, frame: () => { for (const update of pending.splice(0)) update(); } };
  Object.defineProperty(document, 'startViewTransition', {
    configurable: true,
    value: (update: () => void) => {
      record.starts += 1;
      pending.push(update);
    },
  });
  return record;
}

it('5.7.3 theme change: showing the row with a stored theme starts no cross-fade', () => {
  const browser = viewTransitions();
  localStorage.setItem('anima.theme', 'light');
  expect(themeSelect(show()).value).toBe('light');
  expect(browser.starts).toBe(0);
});

it('5.7.3 theme change, MOT-042: the operator\'s choice cross-fades in a view transition; the row and storage do not wait', () => {
  const browser = viewTransitions();
  const control = themeSelect(show());
  choose(control, 'light');
  expect(browser.starts).toBe(1);
  expect(control.value).toBe('light');
  expect(localStorage.getItem('anima.theme')).toBe('light');
  // The browser snapshots the old look, then applies the new theme on its next frame and fades between them.
  expect(document.documentElement.dataset['theme']).toBeUndefined();
  browser.frame();
  expect(document.documentElement.dataset['theme']).toBe('light');
});
