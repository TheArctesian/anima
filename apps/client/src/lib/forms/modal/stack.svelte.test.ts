// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { vi, expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/error';
import { installAnimations } from '../animations.fixture';
import ConfirmModal from './ConfirmModal.svelte';

const OK: HubResult<unknown> = { ok: true, value: undefined };
installAnimations();

/** Two ConfirmModals open together, the second on top; each state's `onCancel` counts Escape. */
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
    onCancel: () => {
      heard[name] += 1;
    },
  });
  const apps = [mount(ConfirmModal, { target: document.body, props: props(outer, 'outer') }), mount(ConfirmModal, { target: document.body, props: props(inner, 'inner') })];
  flushSync();
  return { heard, outer, inner, unmountAll: () => apps.forEach((one) => void unmount(one)) };
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
