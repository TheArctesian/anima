// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import DetailBar from './DetailBar.svelte';

const text = (html: string) => createRawSnippet(() => ({ render: () => html }));
let app: ReturnType<typeof mount> | undefined;
let report: ((entries: Partial<IntersectionObserverEntry>[]) => void) | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  document.title = '';
  Reflect.deleteProperty(window, 'IntersectionObserver');
});

function show(): void {
  Object.defineProperty(window, 'IntersectionObserver', {
    configurable: true,
    value: class {
      constructor(callback: (entries: Partial<IntersectionObserverEntry>[]) => void) {
        report = callback;
      }
      observe = (): undefined => undefined;
      disconnect = (): undefined => undefined;
    },
  });
  app = mount(DetailBar, {
    target: document.body,
    props: { back: { to: '/devices', label: 'Devices' }, icon: 'carbon:bare-metal-server', title: 'nas', reason: text('<span><a href="/x">Open checks</a></span>') },
  });
  flushSync();
}

it('NAV-010, UR-7: the bar names the page in its h1 and the window title, and links back to its list', () => {
  show();
  expect(document.querySelector('h1')?.textContent).toBe('nas');
  expect(document.title).toBe('nas – Anima');
  const back = document.querySelector('nav a');
  expect(back?.textContent).toBe('Devices');
  expect(back?.getAttribute('href')).toMatch(/\/devices$/);
});

it('UR-7: once the bar pins, the folded reason leaves the tab order, and comes back when the bar unpins', () => {
  show();
  const fold = () => document.querySelector<HTMLElement & { inert?: boolean }>('.fold');
  const inert = () => fold()?.inert === true || fold()?.hasAttribute('inert') === true;
  expect(inert()).toBe(false);
  report?.([{ isIntersecting: false, boundingClientRect: { top: -10 } as DOMRectReadOnly }]);
  flushSync();
  expect(inert()).toBe(true);
  report?.([{ isIntersecting: true, boundingClientRect: { top: 40 } as DOMRectReadOnly }]);
  flushSync();
  expect(inert()).toBe(false);
});

it('D-13, MOT-007: with no title yet the bar holds only the back link and a hidden placeholder, and no heading', () => {
  app = mount(DetailBar, { target: document.body, props: { back: { to: '/devices', label: 'Devices' }, icon: 'carbon:bare-metal-server', title: null } });
  flushSync();
  expect(document.querySelector('h1')).toBeNull();
  expect(document.querySelector('nav')?.textContent.trim()).toBe('Devices');
  expect(document.querySelector('.placeholder')?.getAttribute('aria-hidden')).toBe('true');
  expect(document.title).toBe('Anima');
});
