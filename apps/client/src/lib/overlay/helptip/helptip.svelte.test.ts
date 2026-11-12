// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '../../forms/animations.fixture';
import { overlayCovers } from '../overlay.svelte';
import HelpTip from './HelpTip.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
  // jsdom has no `:focus-visible`; a keyboard focus is the case these tests are about.
  vi.spyOn(Element.prototype, 'matches').mockImplementation(function (this: Element, selector: string) {
    return selector === ':focus-visible' || Element.prototype.matches.call(this, selector);
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function tip(): HTMLButtonElement {
  const children = createRawSnippet(() => ({ render: () => '<span>Default is 30 seconds.</span>' }));
  app = mount(HelpTip, { target: document.body, props: { label: 'Timeout', children } });
  flushSync();
  const button = document.querySelector('button');
  if (!button) throw new Error('no button');
  return button;
}

const panel = () => document.querySelector('[role="status"]');

function key(name: string): void {
  document.dispatchEvent(new KeyboardEvent('keydown', { key: name, bubbles: true }));
  flushSync();
}

/** Lets an outro finish: the fake animation ends in a microtask, and the element leaves after it. */
async function settled(): Promise<void> {
  await Promise.resolve();
  await tick();
  flushSync();
}

it('UI-028, UI-015: the trigger is named "About <thing>" and starts closed', () => {
  const button = tip();
  expect(button.getAttribute('aria-label')).toBe('About Timeout');
  expect(button.getAttribute('aria-expanded')).toBe('false');
  expect(panel()).toBeNull();
});

it('UI-028: a click opens the tip with its content, and a second click closes it', () => {
  const button = tip();
  button.click();
  flushSync();
  expect(panel()?.textContent).toBe('Default is 30 seconds.');
  expect(button.getAttribute('aria-expanded')).toBe('true');
  button.click();
  flushSync();
  expect(button.getAttribute('aria-expanded')).toBe('false');
});

it('UI-028: keyboard focus opens the tip', () => {
  const button = tip();
  button.focus();
  flushSync();
  expect(panel()).not.toBeNull();
});

it('WCAG 1.4.13: Escape dismisses the tip and focus is on the button', () => {
  const button = tip();
  button.click();
  flushSync();
  key('Escape');
  expect(button.getAttribute('aria-expanded')).toBe('false');
  expect(document.activeElement).toBe(button);
});

it('WCAG 1.4.13: Escape dismisses a focus-opened tip and it stays closed while focus stays', async () => {
  const button = tip();
  button.focus();
  flushSync();
  key('Escape');
  await settled();
  expect(panel()).toBeNull();
  expect(document.activeElement).toBe(button);
});

it('WCAG 1.4.13: the tip stays open while the pointer is over it, even after focus has left, and closes when the pointer leaves', async () => {
  const button = tip();
  button.focus();
  button.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' }));
  flushSync();
  button.blur();
  flushSync();
  expect(panel()).not.toBeNull();
  button.parentElement?.dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' }));
  flushSync();
  await settled();
  expect(panel()).toBeNull();
});

it('UI-028: a press outside closes a pinned tip; a press inside the tip does not', async () => {
  const button = tip();
  button.click();
  flushSync();
  panel()?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  expect(panel()).not.toBeNull();
  document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  await settled();
  expect(panel()).toBeNull();
});

it('UI-028: focus moving away closes the tip', async () => {
  const button = tip();
  button.focus();
  flushSync();
  button.dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: document.body }));
  flushSync();
  await settled();
  expect(panel()).toBeNull();
});

it('MAT-008, WEB-005: while open the tip covers the panes, and it stops when closed or destroyed', async () => {
  const button = tip();
  expect(overlayCovers()).toBe(false);
  button.click();
  flushSync();
  expect(overlayCovers()).toBe(true);
  key('Escape');
  expect(overlayCovers()).toBe(false);
  button.click();
  flushSync();
  expect(overlayCovers()).toBe(true);
  if (app) await unmount(app);
  app = undefined;
  expect(overlayCovers()).toBe(false);
});
