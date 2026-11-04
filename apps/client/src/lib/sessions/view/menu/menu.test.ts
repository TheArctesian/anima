// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import { KEY_COMBINATIONS, PASTE_STROKES, keyStrokes } from '../keys';
import { connectView, flags, installBrowserFakes, sent, settle, showView, systemClipboard } from '../view.fixture.svelte';
import { menuFocus } from './menu';

describe('menuFocus', () => {
  const items = ['a', 'b', 'c'];
  it.each([
    ['ArrowDown', 'a', 'b'],
    ['ArrowDown', 'c', 'a'],
    ['ArrowUp', 'a', 'c'],
    ['ArrowUp', 'b', 'a'],
    ['ArrowDown', 'none', 'a'],
    ['ArrowUp', 'none', 'c'],
    ['Home', 'b', 'a'],
    ['End', 'a', 'c'],
    ['x', 'a', undefined],
  ])('%s from %s moves to %s', (key, active, expected) => {
    expect(menuFocus(key, items, active)).toBe(expected);
  });
});

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  flags.visible = true;
  flags.focused = true;
  cleanups = [installBlankCanvas(), installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
  vi.useRealTimers();
});

async function connected() {
  const view = showView();
  cleanups.push(view.close);
  const socket = await connectView();
  const trigger = view.target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
  if (trigger === null) throw new Error('no Keys button');
  return { ...view, socket, trigger };
}

const items = () => [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
const strokes = (socket: FakeTunnelSocket) => sent(socket, 'key').map(([keysym, pressed]) => [Number(pressed), Number(keysym)]);

function press(target: Element, key: string) {
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  flushSync();
}

describe('Keys menu (RA-006, D-10)', () => {
  it.each(KEY_COMBINATIONS.map((combination) => [combination.label, combination] as const))(
    'RA-006: "%s" sends its press and release sequence from the keys table',
    async (label, combination) => {
      const { socket, trigger } = await connected();
      trigger.click();
      flushSync();
      items().find((item) => item.textContent.includes(label) && item.querySelector('.label')?.textContent === label)?.click();
      flushSync();
      expect(strokes(socket)).toEqual(keyStrokes(combination).map(([pressed, keysym]) => [pressed, keysym]));
    },
  );

  it('D-9: "Paste" sends the system clipboard as one text/plain stream, then the paste keys', async () => {
    const { socket, trigger } = await connected();
    systemClipboard.text = 'uptime';
    trigger.click();
    flushSync();
    items().at(-1)?.click();
    await settle();
    expect(sent(socket, 'clipboard').map(([, type]) => type)).toEqual(['text/plain']);
    expect(strokes(socket)).toEqual(PASTE_STROKES.map(([pressed, keysym]) => [pressed, keysym]));
  });

});

describe('Keys menu control (UI-015)', () => {
  it('UI-015: the control has a text name, a tooltip and an aria-hidden icon; it says it opens a menu', async () => {
    const { trigger } = await connected();
    expect(trigger.textContent.trim()).toContain('Keys');
    expect(trigger.title).not.toBe('');
    expect(trigger.querySelector('.icon')?.getAttribute('aria-hidden')).toBe('true');
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
  });

  it('keyboard: ArrowDown opens on the first item, arrows move, Escape closes and returns focus', async () => {
    const { trigger } = await connected();
    trigger.focus();
    press(trigger, 'ArrowDown');
    await settle();
    expect(trigger.getAttribute('aria-expanded')).toBe('true');
    expect(document.activeElement).toBe(items()[0]);
    press(document.activeElement as Element, 'ArrowDown');
    expect(document.activeElement).toBe(items()[1]);
    press(document.activeElement as Element, 'ArrowUp');
    press(document.activeElement as Element, 'ArrowUp');
    expect(document.activeElement).toBe(items().at(-1));
    press(document.activeElement as Element, 'Escape');
    await settle();
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
    expect(document.activeElement).toBe(trigger);
  });

  it('D-8: keys pressed inside the menu are not sent to the device', async () => {
    const { socket, trigger } = await connected();
    trigger.focus();
    press(trigger, 'ArrowDown');
    await settle();
    press(document.activeElement as Element, 'ArrowDown');
    press(document.activeElement as Element, 'Escape');
    expect(sent(socket, 'key')).toEqual([]);
  });

  it('the menu cannot be used before the session is ready', () => {
    const view = showView();
    cleanups.push(view.close);
    expect(view.target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.disabled).toBe(true);
  });
});
