// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { RESIZE_SETTLE_MS } from '../tunnel/terminal/link';
import { FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import { KEY_SEQUENCES } from './menu/sequences';
import {
  FakeResizeObserver,
  connectView,
  flags,
  key,
  installBrowserFakes,
  screenText,
  settle,
  showView,
  systemClipboard,
  textareaOf,
} from './view.fixture.svelte';

// The fit add-on measures cells, which jsdom cannot; this one fits to a size the test sets.
const fitted = vi.hoisted(() => ({ cols: 100, rows: 30 }));
vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    terminal: { resize(cols: number, rows: number): void } | undefined;
    activate(terminal: { resize(cols: number, rows: number): void }) {
      this.terminal = terminal;
    }
    dispose() {
      this.terminal = undefined;
    }
    fit() {
      this.terminal?.resize(fitted.cols, fitted.rows);
    }
  },
}));

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTerminalSocket.reset();
  flags.visible = true;
  flags.focused = true;
  Object.assign(fitted, { cols: 100, rows: 30 });
  cleanups = [installBlankCanvas(), installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  vi.useRealTimers();
});

async function connected() {
  const view = showView();
  cleanups.push(view.close);
  const socket = await connectView();
  return { ...view, socket, field: textareaOf(view.target) };
}

const A = { key: 'a', code: 'KeyA', keyCode: 65 };
const C = { key: 'c', code: 'KeyC', keyCode: 67 };
const bytes = (socket: FakeTerminalSocket) => socket.typed();

describe('the socket (K4 §8.1, §8.4)', () => {
  it('K4 §8.1: it opens the ticket path with the token in the query and the subprotocol anima-terminal', async () => {
    const { socket } = await connected();
    expect(socket.url).toBe('wss://hub.anima.ing/terminal?token=TOKEN-MARKER-7f3a-1');
    expect(socket.protocol).toBe('anima-terminal');
    expect(socket.binaryType).toBe('arraybuffer');
  });

  it('K4 §8.4: the first message is resize with the fitted size, before anything typed', async () => {
    const { socket, field } = await connected();
    key(field, A);
    expect(socket.messages()).toEqual([{ type: 'resize', cols: 100, rows: 30 }]);
    expect(socket.frames[0]).toBe('{"type":"resize","cols":100,"rows":30}');
    expect(bytes(socket)).toEqual([97]);
  });

  it('RA-003: a pane resize refits the terminal and sends one resize after the size settles', async () => {
    const { socket } = await connected();
    Object.assign(fitted, { cols: 120, rows: 40 });
    FakeResizeObserver.resize(900, 700);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS - 1);
    expect(socket.messages()).toHaveLength(1);
    Object.assign(fitted, { cols: 121, rows: 41 });
    FakeResizeObserver.resize(910, 710);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS);
    expect(socket.messages().slice(1)).toEqual([{ type: 'resize', cols: 121, rows: 41 }]);
  });

  it('K4 §8.4: the client pings after 5 seconds of sending nothing, and typing delays the ping', async () => {
    const { socket, field } = await connected();
    vi.advanceTimersByTime(4000);
    key(field, A);
    vi.advanceTimersByTime(4000);
    expect(socket.messages()).toHaveLength(1);
    vi.advanceTimersByTime(1000);
    expect(socket.messages().slice(1)).toEqual([{ type: 'ping' }]);
  });

  it('K4 §8.5: 15 seconds without a frame from the hub is a lost connection; a hub ping keeps it alive', async () => {
    const { socket, target, calls } = await connected();
    for (let second = 0; second < 40; second += 10) {
      vi.advanceTimersByTime(10_000);
      socket.control({ type: 'ping' });
    }
    expect(socket.closedByClient).toBeUndefined();
    vi.advanceTimersByTime(15_000);
    await settle();
    expect(socket.closedByClient).toBe(1000);
    expect(calls.endSession).toEqual(['session-1']);
    expect(target.querySelector('[data-reason]')?.textContent).toBe('The connection to the hub was lost.');
  });
});

describe('a flood from the device (K4 §8.5)', () => {
  it('K4 §8.5: output beyond what can be drawn is skipped with one fixed notice; nothing throws and the session goes on', async () => {
    const { socket, target } = await connected();
    const errors: unknown[] = [];
    const caught = (event: ErrorEvent) => errors.push(event.error);
    window.addEventListener('error', caught);
    const chunk = new Uint8Array(4 * 1024 * 1024).fill(0x78);
    for (let frame = 0; frame < 8; frame += 1) socket.output(chunk);
    await settle();
    window.removeEventListener('error', caught);
    expect(errors).toEqual([]);
    const notices = [...target.querySelectorAll('[role="status"]')].filter((entry) => entry.textContent.includes('some was skipped'));
    expect(notices).toHaveLength(1);
    expect(notices[0]?.textContent).toBe('Press Ctrl+L to redraw: output arrived faster than it could be drawn, so some was skipped.');
    expect(socket.closedByClient).toBeUndefined();
    expect(target.querySelector('[data-reason]')).toBeNull();
  });
});

describe('typing and output (RA-003)', () => {
  it('RA-003: what the hub sends is drawn, and a message of another type is ignored', async () => {
    const { socket, target } = await connected();
    socket.output('hello ');
    socket.control({ type: 'mystery' });
    socket.output(new Uint8Array([0xe2, 0x9c, 0x93]));
    expect(screenText(target)).toContain('hello ✓');
  });

  it('RA-003: the terminal\'s own shortcuts are cancelled for the browser, and a paste shortcut is left to the paste event', async () => {
    const { field } = await connected();
    expect(key(field, { ...C, ctrlKey: true, shiftKey: true }).defaultPrevented).toBe(true);
    expect(key(field, { key: 'f', code: 'KeyF', keyCode: 70, metaKey: true }).defaultPrevented).toBe(true);
    expect(key(field, { key: 'v', code: 'KeyV', keyCode: 86, metaKey: true }).defaultPrevented).toBe(false);
  });

  it('RA-003: Ctrl+C sends the interrupt byte; Ctrl+Shift+C is the copy shortcut and sends nothing', async () => {
    const { socket, field } = await connected();
    key(field, { ...C, ctrlKey: true });
    expect(bytes(socket)).toEqual([3]);
    key(field, { ...C, ctrlKey: true, shiftKey: true });
    expect(bytes(socket)).toEqual([3]);
  });

});

describe('the Keys menu and the clipboard (RA-003, RA-006)', () => {
  it.each(KEY_SEQUENCES.map((sequence) => [sequence.label, sequence] as const))(
    'RA-006: "%s" sends its bytes from the Keys menu',
    async (label, sequence) => {
      const { socket, target } = await connected();
      target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
      flushSync();
      const item = [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((entry) => entry.querySelector('.label')?.textContent === label);
      item?.click();
      flushSync();
      expect(bytes(socket)).toEqual([...sequence.bytes]);
    },
  );

  it('RA-006: the Keys menu offers no Ctrl+Alt+Delete, which a shell has no use for', async () => {
    const { target } = await connected();
    target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
    flushSync();
    const labels = [...document.querySelectorAll('[role="menuitem"] .label')].map((label) => label.textContent);
    expect(labels).toEqual(['Ctrl+C', 'Ctrl+D', 'Ctrl+Z', 'Ctrl+L', 'Esc', 'Tab', 'Paste']);
  });

});

describe('copy and paste (RA-003)', () => {
  it('RA-003: "Paste" sends the clipboard as UTF-8, in a bracketed paste when the shell asked for it', async () => {
    const { socket, target } = await connected();
    const paste = async (text: string) => {
      systemClipboard.text = text;
      target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
      flushSync();
      [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].at(-1)?.click();
      await settle();
    };
    await paste('é');
    expect(bytes(socket)).toEqual([0xc3, 0xa9]);
    socket.output('\x1b[?2004h');
    vi.advanceTimersByTime(100);
    await paste('ls');
    const sentText = new TextDecoder().decode(Uint8Array.from(bytes(socket).slice(2)));
    expect(sentText).toBe('\x1b[200~ls\x1b[201~');
  });

  it('RA-003: a paste cannot end the bracketed paste early: an embedded ESC [ 201 ~ loses its escape, from the paste event and the menu', async () => {
    const { socket, target, field } = await connected();
    socket.output('\x1b[?2004h');
    vi.advanceTimersByTime(100);
    const event = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'clipboardData', { value: { getData: () => 'safe\x1b[201~touch /tmp/pwned\n' } });
    field.dispatchEvent(event);
    systemClipboard.text = 'again\x1b[201~rm -rf x';
    target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
    flushSync();
    [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].at(-1)?.click();
    await settle();
    const text = new TextDecoder().decode(Uint8Array.from(socket.typed()));
    expect(text).toBe('\x1b[200~safe[201~touch /tmp/pwned\r\x1b[201~\x1b[200~again[201~rm -rf x\x1b[201~');
    expect(text.split('\x1b[201~')).toHaveLength(3);
  });

});

describe('large paste and copy (RA-003)', () => {
  it('RA-003: a large paste is cut into frames under the 64 KiB limit, in order', async () => {
    const { socket, target } = await connected();
    systemClipboard.text = 'x'.repeat(100_000);
    target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
    flushSync();
    [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].at(-1)?.click();
    await settle();
    const frames = socket.frames.filter((frame): frame is Uint8Array => typeof frame !== 'string');
    expect(frames.length).toBeGreaterThan(1);
    for (const frame of frames) expect(frame.length).toBeLessThanOrEqual(65_536);
    expect(frames.reduce((total, frame) => total + frame.length, 0)).toBe(100_000);
  });

  it('RA-003: a copy the system clipboard refuses shows fixed text, never the clipboard text', async () => {
    const { target, heard } = await connected();
    systemClipboard.refuse = true;
    heard.hooks?.oncopy('SECRET-CLIP');
    await settle();
    expect(target.textContent).toContain('Could not copy the text to your clipboard.');
    expect(target.textContent).not.toContain('SECRET-CLIP');
  });

  it('RA-003: the copy shortcut writes the selection to the system clipboard', async () => {
    const { heard } = await connected();
    heard.hooks?.oncopy('selected text');
    await settle();
    expect(systemClipboard.written).toEqual(['selected text']);
  });
});

describe('keyboard and focus (D-8)', () => {
  it('D-8: the terminal takes the focus when its tab is focused, and gives it up when the app loses focus', async () => {
    const { field } = await connected();
    expect(document.activeElement).toBe(field);
    flags.focused = false;
    flushSync();
    expect(document.activeElement).not.toBe(field);
    flags.focused = true;
    flushSync();
    expect(document.activeElement).toBe(field);
  });

  it('D-8: nothing is sent for a key typed before the socket is open or after the session ended', async () => {
    const view = showView();
    cleanups.push(view.close);
    FakeResizeObserver.resize(800, 600);
    await settle();
    const socket = FakeTerminalSocket.latest();
    key(textareaOf(view.target), A);
    expect(socket.frames).toEqual([]);
    socket.open();
    socket.closeWith(1000);
    await settle();
    key(textareaOf(view.target), A);
    expect(bytes(socket)).toEqual([]);
  });
});
