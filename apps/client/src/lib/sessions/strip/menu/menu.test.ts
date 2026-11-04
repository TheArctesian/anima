// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { placeUnder } from './anchor';
import TabMenu from './TabMenu.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let menu: ReturnType<typeof mount> | undefined;
const onpopout = vi.fn();
const onreturn = vi.fn();
const keyframes: Keyframe[][] = [];
const MENU_WIDTH = 120;

/** A scrolling strip like the real one, with the menu's control inside it. */
function show(ready: boolean, at = new DOMRect(300, 40, 32, 32)): HTMLElement {
  const strip = document.createElement('div');
  strip.style.overflowX = 'auto';
  document.body.append(strip);
  menu = mount(TabMenu, { target: strip, props: { label: 'More for web-1 SSH session', ready, onpopout, onreturn } });
  flushSync();
  const control = strip.querySelector<HTMLElement>('[aria-haspopup="menu"]');
  if (control === null) throw new Error('no menu button');
  control.getBoundingClientRect = () => at;
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
  keyframes.length = 0;
  const record = (frames: Keyframe[]) => {
    keyframes.push(frames);
    return animation;
  };
  Object.defineProperty(Element.prototype, 'animate', { value: record, configurable: true });
  // A menu measures where its style puts it: MENU_WIDTH wide, 40 px high.
  const measure = Element.prototype.getBoundingClientRect;
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
    if (!(this instanceof HTMLElement) || this.getAttribute('role') !== 'menu') return measure.call(this);
    return new DOMRect(Number.parseFloat(this.style.left) || 0, Number.parseFloat(this.style.top) || 0, MENU_WIDTH, 40);
  });
});

afterEach(async () => {
  if (menu !== undefined) await unmount(menu);
  menu = undefined;
  document.body.innerHTML = '';
  Reflect.deleteProperty(Element.prototype, 'animate');
  vi.restoreAllMocks();
});

it.each([
  ['fits: aligned to the control', 300, 300],
  ['near the right edge: moved left to stay inside', 1380, 1440 - 120 - 8],
  ['wider than the room on the left: kept off the left edge', 2, 8],
])('UI-009: a menu that %s', (_name, controlLeft, left) => {
  expect(placeUnder(new DOMRect(controlLeft, 40, 32, 32), 120, 1440, 8)).toEqual({ left, top: 72 });
});

it('UI-009 / MOT-005: a control near the right edge gives a menu inside the viewport, growing from the control', async () => {
  const width = window.innerWidth;
  const control = show(true, new DOMRect(width - 40, 40, 32, 32));
  control.click();
  await settle();
  const list = document.querySelector<HTMLElement>('[role="menu"]');
  const box = list?.getBoundingClientRect();
  expect(box?.right).toBeLessThanOrEqual(width);
  expect(box?.left).toBe(width - MENU_WIDTH);
  // The control's centre (width - 24, 56) relative to the menu's corner (width - 120, 72).
  const origin = keyframes.flat().find((frame) => 'transformOrigin' in frame)?.transformOrigin;
  expect(origin).toBe(`${String(MENU_WIDTH - 24)}px -16px`);
});

it('UI-015: a press on the control never puts focus inside it, and a toggle-close returns focus to the tab', async () => {
  const control = show(true);
  const down = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
  control.dispatchEvent(down);
  expect(down.defaultPrevented).toBe(true);
  control.focus();
  expect(onreturn).toHaveBeenCalledOnce();
  control.click();
  await settle();
  control.click();
  await settle();
  expect(document.querySelector('[role="menu"]')).toBeNull();
  expect(onreturn).toHaveBeenCalledTimes(2);
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
