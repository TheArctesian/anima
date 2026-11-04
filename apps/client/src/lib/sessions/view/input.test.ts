// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../tunnel/canvas.fixture';
import { CLIPBOARD_LIMIT, receiveClipboard } from '../tunnel/input';
import { FakeTunnelSocket } from '../tunnel/socket.fixture';
import type { SessionTunnel } from '../tunnel/tunnel';
import { KEYSYM, PASTE_STROKES } from './keys';
import { TOKEN_MARKER, connectView, flags, installBrowserFakes, paneOf, sent, settle, showView, systemClipboard } from './view.fixture.svelte';

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

  it('D-9: clipboard text that arrives while hidden is written when shown again, latest value only', async () => {
    const { socket } = await connected();
    flags.visible = false;
    flushSync();
    socket.clipboard('first', 1);
    socket.clipboard('second', 2);
    await settle();
    expect(systemClipboard.written).toEqual([]);
    flags.visible = true;
    flushSync();
    await settle();
    expect(systemClipboard.written).toEqual(['second']);
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

describe('LOG-007', () => {
  it('LOG-007: no console output in a whole session holds the token, clipboard text or keystrokes', async () => {
    const spies = (['log', 'info', 'warn', 'error', 'debug', 'trace'] as const).map((level) =>
      vi.spyOn(console, level).mockImplementation(() => undefined),
    );
    const { socket, pane, target } = await connected();
    socket.clipboard('CLIP-MARKER-91c2');
    typeEnter(pane);
    const event = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'clipboardData', { value: { getData: () => 'PASTE-MARKER-4b1e' } });
    pane.dispatchEvent(event);
    socket.gatewayError(0x0301);
    socket.closeWith(1011);
    await settle();
    const logged = JSON.stringify(spies.flatMap((spy) => spy.mock.calls));
    for (const marker of [TOKEN_MARKER, 'CLIP-MARKER-91c2', 'PASTE-MARKER-4b1e', String(ENTER), 'Aborted']) {
      expect(logged).not.toContain(marker);
    }
    expect(document.body.innerHTML).not.toContain(TOKEN_MARKER);
    expect(target.textContent).not.toContain('CLIP-MARKER-91c2');
    for (const spy of spies) spy.mockRestore();
  });
});
