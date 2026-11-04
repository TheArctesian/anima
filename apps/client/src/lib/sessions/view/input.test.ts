// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { CLIPBOARD_LIMIT, receiveClipboard } from '../tunnel/input';
import { FakeTunnelSocket } from '../tunnel/fake/socket.fixture';
import type { SessionTunnel } from '../tunnel/tunnel';
import { KEYSYM, PASTE_STROKES } from './keys';
import { connectView, flags, installBrowserFakes, paneOf, sent, settle, showView, systemClipboard } from './view.fixture.svelte';

const ENTER = 0xff0d;
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
  return { ...view, socket, pane: paneOf(view.target) };
}

function key(target: EventTarget, type: 'keydown' | 'keyup', init: KeyboardEventInit & { keyCode?: number }) {
  const event = new KeyboardEvent(type, { bubbles: true, cancelable: true, ...init });
  if (init.keyCode !== undefined) Object.defineProperty(event, 'keyCode', { value: init.keyCode });
  target.dispatchEvent(event);
  return event;
}

function typeEnter(target: EventTarget) {
  key(target, 'keydown', { key: 'Enter', code: 'Enter', keyCode: 13 });
  key(target, 'keyup', { key: 'Enter', code: 'Enter', keyCode: 13 });
}

const strokes = (socket: FakeTunnelSocket) => sent(socket, 'key').map(([keysym, pressed]) => [Number(pressed), Number(keysym)]);

describe('keyboard (D-8)', () => {
  it('D-8: a key on the focused, visible pane reaches the tunnel as a press 1 and a release 0', async () => {
    const { socket, pane } = await connected();
    expect(document.activeElement).toBe(pane);
    typeEnter(pane);
    expect(sent(socket, 'key')).toEqual([
      [String(ENTER), '1'],
      [String(ENTER), '0'],
    ]);
  });

  it('D-8: keys on the document, outside the pane, never reach the tunnel', async () => {
    const { socket } = await connected();
    typeEnter(document.body);
    typeEnter(document);
    expect(sent(socket, 'key')).toEqual([]);
  });

  it.each([
    ['hidden', () => (flags.visible = false)],
    ['not focused', () => (flags.focused = false)],
  ])('D-8: no key reaches the tunnel while the pane is %s', async (_name, change) => {
    const { socket, pane } = await connected();
    change();
    flushSync();
    typeEnter(pane);
    expect(sent(socket, 'key')).toEqual([]);
  });

  it('D-8: no key reaches the tunnel after the view is closed', async () => {
    const { socket, pane, close } = await connected();
    await close();
    typeEnter(pane);
    expect(sent(socket, 'key')).toEqual([]);
  });

});

describe('keyboard across hide and show (D-8)', () => {
  it('D-8: a key held when the tab is hidden is released once, so the device has no stuck key, and then nothing', async () => {
    const { socket, pane } = await connected();
    key(pane, 'keydown', { key: 'Control', code: 'ControlLeft', keyCode: 17, location: 1, ctrlKey: true });
    flags.visible = false;
    flushSync();
    vi.advanceTimersByTime(2000);
    key(pane, 'keyup', { key: 'Control', code: 'ControlLeft', keyCode: 17, location: 1 });
    expect(strokes(socket)).toEqual([
      [1, KEYSYM.controlLeft],
      [0, KEYSYM.controlLeft],
    ]);
  });

  it('D-8: the keyboard binds again when the tab is shown again', async () => {
    const { socket, pane } = await connected();
    flags.visible = false;
    flushSync();
    flags.visible = true;
    flushSync();
    typeEnter(pane);
    expect(strokes(socket)).toEqual([
      [1, ENTER],
      [0, ENTER],
    ]);
  });

  it('D-9: with Command held, keys go to the browser (paste), not to the device', async () => {
    const { socket, pane } = await connected();
    key(pane, 'keydown', { key: 'Meta', code: 'MetaLeft', keyCode: 91, location: 1, metaKey: true });
    const v = key(pane, 'keydown', { key: 'v', code: 'KeyV', keyCode: 86, metaKey: true });
    expect(v.defaultPrevented).toBe(false);
    expect(sent(socket, 'key')).toEqual([]);
  });
});

/**
 * Press Backspace on the pane, do `leave`, release it outside the pane (where the pane's keyboard cannot hear it),
 * and give the key-repeat timer 10 s: only the listener for `leave` can release the key on the device.
 */
async function holdAndLeave(leave: (pane: HTMLElement, trigger: HTMLElement) => void) {
  const { socket, pane, target } = await connected();
  key(pane, 'keydown', { key: 'Backspace', code: 'Backspace', keyCode: 8 });
  const trigger = target.querySelector<HTMLElement>('[aria-haspopup="menu"]');
  if (trigger === null) throw new Error('no Keys button');
  leave(pane, trigger);
  flushSync();
  key(document.body, 'keyup', { key: 'Backspace', code: 'Backspace', keyCode: 8 });
  vi.advanceTimersByTime(10_000);
  return { socket, pane, strokes: strokes(socket) };
}

const BACKSPACE = 0xff08;

describe('keyboard and DOM focus (D-8, SEC-M1)', () => {
  it.each([
    ['focus moves to the Keys control', (_pane: HTMLElement, trigger: HTMLElement) => trigger.focus()],
    ['the window loses focus', () => window.dispatchEvent(new FocusEvent('blur'))],
    [
      'the page is hidden',
      () => {
        Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
        document.dispatchEvent(new Event('visibilitychange'));
      },
    ],
  ])('D-8: a key held when %s is released once and does not repeat', async (_name, leave) => {
    const { strokes: heard } = await holdAndLeave(leave);
    Reflect.deleteProperty(document, 'visibilityState');
    expect(heard).toEqual([
      [1, BACKSPACE],
      [0, BACKSPACE],
    ]);
  });

  it('D-8: keys on the pane are not sent while the window has no focus, and are again once it has', async () => {
    const { socket, pane } = await connected();
    window.dispatchEvent(new FocusEvent('blur'));
    flushSync();
    typeEnter(pane);
    expect(sent(socket, 'key')).toEqual([]);
    window.dispatchEvent(new FocusEvent('focus'));
    flushSync();
    typeEnter(pane);
    expect(strokes(socket)).toEqual([
      [1, ENTER],
      [0, ENTER],
    ]);
  });
});

describe('paste shortcuts (D-9, SEC-S2)', () => {
  it('D-9: after Ctrl+Shift+V with Ctrl still held, the device still holds Ctrl, so the next Ctrl+C is Ctrl+C', async () => {
    const { socket, pane } = await connected();
    const ctrl = { key: 'Control', code: 'ControlLeft', keyCode: 17, location: 1, ctrlKey: true };
    const shift = { key: 'Shift', code: 'ShiftLeft', keyCode: 16, location: 1, ctrlKey: true, shiftKey: true };
    const v = { key: 'V', code: 'KeyV', keyCode: 86, ctrlKey: true, shiftKey: true };
    key(pane, 'keydown', ctrl);
    key(pane, 'keydown', shift);
    key(pane, 'keydown', v);
    const paste = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(paste, 'clipboardData', { value: { getData: () => 'echo hi' } });
    pane.dispatchEvent(paste);
    key(pane, 'keyup', v);
    key(pane, 'keyup', { ...shift, shiftKey: false });
    key(pane, 'keydown', { key: 'c', code: 'KeyC', keyCode: 67, ctrlKey: true });
    const heard = strokes(socket);
    const c = heard.findIndex(([pressed, keysym]) => pressed === 1 && keysym === 0x63);
    const lastCtrl = heard.slice(0, c).filter(([, keysym]) => keysym === 0xffe3).at(-1);
    expect(c).toBeGreaterThan(-1);
    expect(lastCtrl).toEqual([1, 0xffe3]);
    // The paste keys pressed and released only V: Ctrl and Shift were already down on the device.
    expect(heard.filter(([, keysym]) => keysym === 0x56)).toEqual([
      [1, 0x56],
      [0, 0x56],
    ]);
  });

  const CTRL = { key: 'Control', code: 'ControlLeft', keyCode: 17, location: 1, ctrlKey: true };
  const SHIFT = { key: 'Shift', code: 'ShiftLeft', keyCode: 16, location: 1, shiftKey: true };

  it.each([
    ['Ctrl+Shift+V', [CTRL, { ...SHIFT, ctrlKey: true }], { key: 'V', code: 'KeyV', keyCode: 86, ctrlKey: true, shiftKey: true }],
    ['Shift+Insert', [SHIFT], { key: 'Insert', code: 'Insert', keyCode: 45, shiftKey: true }],
  ])('D-9: %s goes to the browser (its paste event), not to guacd', async (_name, modifiers, last) => {
    const { socket, pane } = await connected();
    for (const modifier of modifiers) key(pane, 'keydown', modifier);
    const before = sent(socket, 'key').length;
    const shortcut = key(pane, 'keydown', last);
    expect(shortcut.defaultPrevented).toBe(false);
    expect(sent(socket, 'key')).toHaveLength(before);
  });
});

describe('mouse and clipboard (RA-003)', () => {
  it('RA-003: the wheel on the display sends scroll button presses (guacd scroll-back)', async () => {
    const { socket, target } = await connected();
    const display = target.querySelector('[data-display]')?.firstElementChild;
    display?.dispatchEvent(new WheelEvent('wheel', { deltaY: 120, deltaMode: 0, bubbles: true, cancelable: true }));
    const masks = sent(socket, 'mouse').map(([, , mask]) => Number(mask));
    expect(masks).toContain(16);
    expect(masks.at(-1)).toBe(0);
  });

  it('RA-003: gateway clipboard text reaches the system clipboard', async () => {
    const { socket } = await connected();
    socket.clipboard('copied from the device ✓');
    await settle();
    expect(systemClipboard.written).toEqual(['copied from the device ✓']);
  });

  it('D-9: clipboard text that arrives while the pane is hidden or unfocused is dropped, also once it is shown', async () => {
    const { socket, target } = await connected();
    flags.visible = false;
    flushSync();
    socket.clipboard('while hidden', 1);
    flags.visible = true;
    flushSync();
    target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.focus();
    socket.clipboard('while unfocused', 2);
    await settle();
    expect(systemClipboard.written).toEqual([]);
  });

  it('RA-003: a paste event on the pane sends one text/plain clipboard stream, then the paste keys', async () => {
    const { socket, pane } = await connected();
    const event = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'clipboardData', { value: { getData: (type: string) => (type === 'text/plain' ? 'ls -la' : '<b>x</b>') } });
    pane.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    const streams = sent(socket, 'clipboard');
    expect(streams).toEqual([[expect.any(String), 'text/plain']]);
    const [index] = streams[0] ?? [];
    const blobs = sent(socket, 'blob').filter(([stream]) => stream === index);
    expect(blobs.map(([, data]) => atob(data ?? ''))).toEqual(['ls -la']);
    expect(sent(socket, 'end')).toEqual([[index]]);
    expect(strokes(socket)).toEqual(PASTE_STROKES.map(([pressed, keysym]) => [pressed, keysym]));
  });

  it('a copy the system clipboard refuses shows fixed text, never the clipboard text', async () => {
    const { socket, target } = await connected();
    systemClipboard.refuse = true;
    socket.clipboard('SECRET-CLIP');
    await settle();
    expect(target.textContent).toContain('Could not copy the text to your clipboard.');
    expect(target.textContent).not.toContain('SECRET-CLIP');
  });
});

describe('receiveClipboard (D-9)', () => {
  interface Stream {
    onblob?: (data: string) => void;
    onend?: () => void;
  }
  function fakeClient() {
    const client = { onclipboard: null as null | ((stream: Stream, mimetype: string) => void) };
    return { client, typed: client as unknown as SessionTunnel['client'] };
  }
  const base64 = (text: string) => btoa(String.fromCharCode(...new TextEncoder().encode(text)));

  it.each([
    ['text/plain', ['abc']],
    ['text/plain;charset=utf-8', ['abc']],
    ['text/html', []],
  ])('D-9: a %s stream gives its joined text, other types nothing', (mimetype, expected) => {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    const stream: Stream = {};
    client.onclipboard?.(stream, mimetype);
    stream.onblob?.(base64('ab'));
    stream.onblob?.(base64('c'));
    stream.onend?.();
    expect(heard).toEqual(expected);
  });

  it('D-9: text over the limit is dropped whole', () => {
    const { client, typed } = fakeClient();
    const heard: string[] = [];
    receiveClipboard(typed, (text) => heard.push(text));
    const stream: Stream = {};
    client.onclipboard?.(stream, 'text/plain');
    const chunk = base64('x'.repeat(6144));
    for (let sentLength = 0; sentLength <= CLIPBOARD_LIMIT; sentLength += 6144) stream.onblob?.(chunk);
    stream.onend?.();
    expect(heard).toEqual([]);
  });
});
