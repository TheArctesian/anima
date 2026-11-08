// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { vi, expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import { installAnimations } from '../animations.fixture';
import ConfirmModal from './ConfirmModal.svelte';

const OK: HubResult<unknown> = { ok: true, value: undefined };
installAnimations();

/** Two ConfirmModals open together, the second on top; Escape counts in `heard` and closes the dialog. */
function stacked() {
  const heard = { outer: 0, inner: 0 };
  const outer = $state({ open: true });
  const inner = $state({ open: true });
  const props = (state: { open: boolean }, name: 'outer' | 'inner') => ({
    get open() {
      return state.open;
    },
    title: `${name} dialog`,
    body: 'Sure?',
    confirmLabel: 'Confirm',
    onConfirm: () => Promise.resolve(OK),
    // Like the real owners, Escape closes the dialog.
    onCancel: () => {
      heard[name] += 1;
      state.open = false;
    },
  });
  // Mounted in a container, as the app is: a dialog is moved to `body`, so its events reach Svelte's handler on the document.
  const target = document.body.appendChild(document.createElement('div'));
  const apps = [mount(ConfirmModal, { target, props: props(outer, 'outer') }), mount(ConfirmModal, { target, props: props(inner, 'inner') })];
  // A browser runs Svelte's effects between two listeners of one event; this listener, added before the
  // dialogs' own document listeners exist, does the same here.
  const checkpoint = () => {
    flushSync();
  };
  document.addEventListener('keydown', checkpoint);
  flushSync();
  return {
    heard,
    outer,
    inner,
    unmountAll: () => {
      document.removeEventListener('keydown', checkpoint);
      apps.forEach((one) => void unmount(one));
    },
  };
}

const escapeOnPage = () => {
  (document.activeElement as HTMLElement | null)?.blur();
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  flushSync();
};

it('SEC-007: with two dialogs open, Escape on the page reaches only the topmost; once it closes, the one below answers', () => {
  const { heard, inner, unmountAll } = stacked();
  escapeOnPage();
  expect(heard).toEqual({ outer: 0, inner: 1 });
  inner.open = false;
  flushSync();
  escapeOnPage();
  expect(heard).toEqual({ outer: 1, inner: 1 });
  unmountAll();
});

it('SEC-007: a dialog that is closed or unmounted no longer answers Escape or pulls focus', () => {
  const { heard, outer, inner, unmountAll } = stacked();
  inner.open = false;
  outer.open = false;
  flushSync();
  escapeOnPage();
  expect(heard).toEqual({ outer: 0, inner: 0 });
  unmountAll();
  const added = vi.spyOn(document, 'addEventListener');
  const removed = vi.spyOn(document, 'removeEventListener');
  const again = stacked();
  again.unmountAll();
  for (const type of ['keydown', 'focusin']) {
    const handlers = (calls: typeof added.mock.calls) => calls.filter(([name]) => name === type).map(([, handler]) => handler);
    expect(handlers(removed.mock.calls)).toEqual(expect.arrayContaining(handlers(added.mock.calls)));
  }
  vi.restoreAllMocks();
});

it('SEC-007: focus that lands outside the topmost dialog goes back into it, and not into one below', () => {
  const { inner, unmountAll } = stacked();
  const page = document.createElement('button');
  document.body.append(page);
  page.focus();
  const dialogs = [...document.querySelectorAll('[role="dialog"]')];
  expect(dialogs.at(-1)?.contains(document.activeElement)).toBe(true);
  inner.open = false;
  flushSync();
  page.focus();
  expect(dialogs[0]?.contains(document.activeElement)).toBe(true);
  unmountAll();
  page.focus();
  expect(document.activeElement).toBe(page);
});

it('SEC-007: one Escape with the focus in the inner dialog closes only the inner one', () => {
  const { heard, outer, inner, unmountAll } = stacked();
  const dialogs = [...document.querySelectorAll('[role="dialog"]')];
  expect(dialogs.at(-1)?.contains(document.activeElement)).toBe(true);
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  flushSync();
  expect(heard).toEqual({ outer: 0, inner: 1 });
  expect([outer.open, inner.open]).toEqual([true, false]);
  unmountAll();
});
