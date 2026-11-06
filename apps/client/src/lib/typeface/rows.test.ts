// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import TypefaceRows from './TypefaceRows.svelte';

let app: ReturnType<typeof mount> | undefined;

function show(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(TypefaceRows, { target });
  flushSync();
  return target;
}

function select(target: HTMLElement, label: string): HTMLSelectElement {
  const found = [...target.querySelectorAll('label')].find((each) => each.querySelector('span')?.textContent === label);
  const control = found?.querySelector('select');
  if (control == null) throw new Error(`no ${label}`);
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
  delete document.documentElement.dataset['fontUi'];
  delete document.documentElement.dataset['fontMono'];
  vi.restoreAllMocks();
});

it('UI-020: each text role has its own labelled choice of Fira Code or OpenDyslexic, Fira Code first', () => {
  const target = show();
  for (const label of ['Interface font', 'Monospace font']) {
    const control = select(target, label);
    expect([...control.options].map((option) => option.textContent)).toEqual(['Fira Code', 'OpenDyslexic']);
    expect(control.value).toBe('fira-code');
  }
});

it('UI-020: choosing OpenDyslexic for the interface changes only the interface font, now and after a restart', async () => {
  const target = show();
  choose(select(target, 'Interface font'), 'opendyslexic');
  expect(document.documentElement.dataset).toMatchObject({ fontUi: 'opendyslexic', fontMono: 'fira-code' });
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  const again = show();
  expect(select(again, 'Interface font').value).toBe('opendyslexic');
  expect(select(again, 'Monospace font').value).toBe('fira-code');
});

it('UI-020: a storage that refuses to save still applies the font and says it will not last', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('quota');
  });
  const target = show();
  choose(select(target, 'Monospace font'), 'opendyslexic');
  expect(document.documentElement.dataset['fontMono']).toBe('opendyslexic');
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(
    'Anima could not save this choice on this device; it lasts until the app restarts.',
  );
});

it('UI-020: the "could not save" message goes once a later choice is saved', () => {
  const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementationOnce(() => {
    throw new Error('quota');
  });
  const target = show();
  choose(select(target, 'Monospace font'), 'opendyslexic');
  expect(target.querySelector('[role="alert"]')).not.toBeNull();
  choose(select(target, 'Monospace font'), 'fira-code');
  expect(setItem).toHaveBeenCalledTimes(2);
  expect(target.querySelector('[role="alert"]')).toBeNull();
});
