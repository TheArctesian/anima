// @vitest-environment jsdom
import { expect, it } from 'vitest';
import { watchIconOnly } from './icon-only';

/** A window whose width can change, with the media query answered from it. */
function windowOf(width: number) {
  const listeners = new Set<(event: MediaQueryListEvent) => void>();
  let current = width;
  const matches = () => current < 768;
  return {
    resize(next: number) {
      current = next;
      for (const listener of listeners) listener({ matches: matches() } as MediaQueryListEvent);
    },
    matchMedia: () =>
      ({
        get matches() {
          return matches();
        },
        addEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) => listeners.add(listener),
        removeEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) => listeners.delete(listener),
      }) as unknown as MediaQueryList,
  };
}

it('NAV-001: below bp-compact the root is marked icon-only before anything renders, and follows the window', () => {
  const root = document.createElement('html');
  const win = windowOf(360);
  const watch = watchIconOnly(root, win, false);
  expect(root.getAttribute('data-nav')).toBe('icons');
  win.resize(768);
  expect(root.hasAttribute('data-nav')).toBe(false);
  win.resize(500);
  expect(root.getAttribute('data-nav')).toBe('icons');
  watch.stop();
  win.resize(1200);
  expect(root.getAttribute('data-nav')).toBe('icons');
});

it('NAV-001: a window without matchMedia keeps the full bar', () => {
  const root = document.createElement('html');
  watchIconOnly(root, {}, false).stop();
  expect(root.hasAttribute('data-nav')).toBe(false);
});

it('NAV-006: a collapsed bar shows icons at any width, and expanding it gives the width back to the window', () => {
  const root = document.createElement('html');
  const win = windowOf(1200);
  const heard: { iconOnly: boolean; compact: boolean }[] = [];
  const watch = watchIconOnly(root, win, true, (state) => heard.push(state));
  expect(root.getAttribute('data-nav')).toBe('icons');
  watch.collapse(false);
  expect(root.hasAttribute('data-nav')).toBe(false);
  win.resize(360);
  expect(root.getAttribute('data-nav')).toBe('icons');
  expect(heard.at(-1)).toEqual({ iconOnly: true, compact: true });
  watch.collapse(true);
  win.resize(1200);
  expect(root.getAttribute('data-nav')).toBe('icons');
  expect(heard.at(-1)).toEqual({ iconOnly: true, compact: false });
});
