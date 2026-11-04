// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import TabMenu from './TabMenu.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let menu: ReturnType<typeof mount> | undefined;
const onpopout = vi.fn();
const onreturn = vi.fn();

/** A scrolling strip like the real one, with the menu's control inside it. */
function show(ready: boolean): HTMLElement {
  const strip = document.createElement('div');
  strip.style.overflowX = 'auto';
  document.body.append(strip);
  menu = mount(TabMenu, { target: strip, props: { label: 'More for web-1 SSH session', ready, onpopout, onreturn } });
  flushSync();
  const control = strip.querySelector<HTMLElement>('[aria-haspopup="menu"]');
  if (control === null) throw new Error('no menu button');
  control.getBoundingClientRect = () => new DOMRect(300, 40, 32, 32);
  return control;
}

async function settle(): Promise<void> {
  for (let tick = 0; tick < 4; tick += 1) {
    await Promise.resolve();
    flushSync();
  }
}

beforeEach(() => {
  onpopout.mockClear();
  onreturn.mockClear();
  // jsdom has no Web Animations; this one finishes at once so the menu's exit completes.
  const animation = {
    cancel: () => undefined,
    currentTime: 0,
    playState: 'finished',
    effect: null,
    set onfinish(done: () => void) {
      queueMicrotask(done);
    },
  };
  Object.defineProperty(Element.prototype, 'animate', { value: () => animation, configurable: true });
});

afterEach(async () => {
  if (menu !== undefined) await unmount(menu);
  menu = undefined;
  document.body.innerHTML = '';
  Reflect.deleteProperty(Element.prototype, 'animate');
});

it('UI-009 / MOT-007: the open menu lives outside the scrolling strip, fixed under its control', async () => {
  const control = show(true);
  control.click();
  await settle();
  const list = document.querySelector<HTMLElement>('[role="menu"]');
  expect(list?.parentElement).toBe(document.body);
  expect(control.parentElement?.contains(list ?? null)).toBe(false);
  expect([list?.style.left, list?.style.top]).toEqual(['300px', '72px']);
});

it('UI-015: the control has a name and a tooltip, but is not a Tab stop and is hidden from the tablist', () => {
  const control = show(true);
  expect(control.getAttribute('aria-label')).toBe('More for web-1 SSH session');
  expect(control.getAttribute('title')).toBe('More for web-1 SSH session');
  expect(control.tabIndex).toBe(-1);
  expect(control.getAttribute('aria-hidden')).toBe('true');
});

it('"Pop out" pops the session out and closes the menu', async () => {
  show(true).click();
  await settle();
  document.querySelector<HTMLElement>('[role="menuitem"]')?.click();
  await settle();
  expect(onpopout).toHaveBeenCalledOnce();
  expect(document.querySelector('[role="menu"]')).toBeNull();
});

it('before the session starts, "Pop out" is focusable but disabled and does nothing', async () => {
  show(false).click();
  await settle();
  const item = document.querySelector<HTMLElement>('[role="menuitem"]');
  expect(item?.getAttribute('aria-disabled')).toBe('true');
  expect(document.activeElement).toBe(item);
  item?.click();
  expect(onpopout).not.toHaveBeenCalled();
});

it('Escape closes the menu wherever focus is and sends focus back to the tab', async () => {
  show(false).click();
  await settle();
  document.body.focus();
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
  await settle();
  expect(document.querySelector('[role="menu"]')).toBeNull();
  expect(onreturn).toHaveBeenCalledOnce();
});

it('a press outside, a scroll or a resize closes the menu', async () => {
  for (const close of [
    () => document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })),
    () => document.dispatchEvent(new Event('scroll')),
    () => window.dispatchEvent(new Event('resize')),
  ]) {
    document.body.innerHTML = '';
    show(true).click();
    await settle();
    close();
    await settle();
    expect(document.querySelector('[role="menu"]')).toBeNull();
    if (menu !== undefined) await unmount(menu);
    menu = undefined;
  }
});
