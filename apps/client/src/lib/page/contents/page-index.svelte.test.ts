// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import PageIndex from './PageIndex.svelte';

const ENTRIES = [
  { id: 'one', title: 'One', icon: 'carbon:settings' },
  { id: 'two', title: 'Two', icon: 'carbon:settings' },
  { id: 'three', title: 'Three', icon: 'carbon:settings' },
] as const;

/** Where each section's head sits, in pixels from the top of the window. */
const tops: Record<string, number> = {};

afterEach(() => {
  document.body.innerHTML = '';
  Reflect.deleteProperty(document, 'scrollingElement');
  vi.unstubAllGlobals();
});

function page(): () => void {
  vi.stubGlobal('innerHeight', 900);
  for (const entry of ENTRIES) {
    const section = document.createElement('section');
    section.dataset['section'] = entry.id;
    section.getBoundingClientRect = () => ({ top: tops[entry.id] ?? 0 }) as DOMRect;
    document.body.append(section);
  }
  const target = document.createElement('div');
  document.body.prepend(target);
  const view = mount(PageIndex, { target, props: { entries: ENTRIES, label: 'Sections' } });
  flushSync();
  return () => {
    unmount(view);
  };
}

const current = (): string | undefined => document.querySelector('a[aria-current="true"]')?.textContent.trim();

it('SET-001, D17: the current section is the last whose head has passed the top tenth of the window, as the page scrolls', () => {
  Object.assign(tops, { one: 100, two: 700, three: 1400 });
  const close = page();
  expect(current()).toBe('One');
  Object.assign(tops, { one: -500, two: 60, three: 900 });
  document.dispatchEvent(new Event('scroll'));
  flushSync();
  expect(current()).toBe('Two');
  close();
});

it('SET-001, D17: at the foot of the page the last section is current, though its head never reaches the top', () => {
  Object.assign(tops, { one: -1200, two: -300, three: 500 });
  // The page is scrolled as far as it goes: its foot is the window's foot.
  Object.defineProperty(document, 'scrollingElement', { configurable: true, value: { scrollTop: 1000, scrollHeight: 1900 } });
  const close = page();
  expect(current()).toBe('Three');
  close();
});
