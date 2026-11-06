// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SessionTransfer } from '../../api';
import type { TerminalScreen } from '../../screen/screen';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTerminalSocket } from '../../tunnel/terminal/fake/socket.fixture';
import { connectView, flags, installBrowserFakes, key, settle, showView, systemClipboard, textareaOf } from '../view.fixture.svelte';

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTerminalSocket.reset();
  flags.visible = true;
  flags.focused = true;
  cleanups = [installBlankCanvas(), installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  vi.useRealTimers();
});

const ON: SessionTransfer = { copy: true, paste: true };
const OFF: SessionTransfer = { copy: false, paste: false };
const UNREAD_NOTICE = 'Copy and paste are off: the hub did not say whether this device allows them.';
const C = { key: 'c', code: 'KeyC', keyCode: 67 };

/** An SSH view whose device answers `transfer` at the start, connected, with its screen. */
async function connected(transfer: SessionTransfer | 'unanswered') {
  let screen: TerminalScreen | undefined;
  const view = showView((open) => async (host, hooks) => (screen = await open(host, hooks)));
  view.answers.transfer = transfer;
  cleanups.push(view.close);
  const socket = await connectView();
  if (screen === undefined) throw new Error('no screen');
  return { ...view, socket, screen, field: textareaOf(view.target) };
}

/** Draw `text` and select it, as the operator would with the mouse. */
function selected(socket: FakeTerminalSocket, screen: TerminalScreen, text: string) {
  socket.output(text);
  vi.advanceTimersByTime(100);
  expect(screen.find(text, false, false)).toBe(true);
}

/** A clipboard event carrying `text`, and the spy that hears what a listener writes into it. */
function clipboardEvent(type: string, text = '') {
  const event = new Event(type, { bubbles: true, cancelable: true });
  const setData = vi.fn();
  Object.defineProperty(event, 'clipboardData', { value: { getData: () => text, setData } });
  return { event, setData };
}

function dropEvent(text: string) {
  const event = new Event('drop', { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'dataTransfer', { value: { getData: () => text } });
  return event;
}

const sent = (socket: FakeTerminalSocket) => new TextDecoder().decode(Uint8Array.from(socket.typed()));

function openKeysMenu(target: HTMLElement): HTMLButtonElement | undefined {
  target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
  flushSync();
  return [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].at(-1);
}

describe('copy out of the terminal (RA-033, K4 §8.7)', () => {
  it.each([
    ['on', ON, ['needle']],
    ['off', { copy: false, paste: true }, []],
  ])('copy %s: the copy shortcut with a selection writes it to the system clipboard only when copy is on', async (_, transfer, written) => {
    const { socket, screen, field } = await connected(transfer);
    selected(socket, screen, 'needle');
    key(field, { ...C, ctrlKey: true, shiftKey: true });
    key(field, { ...C, metaKey: true });
    await settle();
    expect(systemClipboard.written).toEqual(written.length === 0 ? [] : [...written, ...written]);
  });

  it('copy on: a copy event (Edit or context menu) gets the selection from the terminal', async () => {
    const { socket, screen, field } = await connected(ON);
    selected(socket, screen, 'needle');
    const { event, setData } = clipboardEvent('copy');
    field.dispatchEvent(event);
    expect(setData).toHaveBeenCalledWith('text/plain', 'needle');
  });

  it.each(['copy', 'cut'])('copy off: a %s event is cancelled before the terminal writes the selection into it', async (type) => {
    const { socket, screen, field } = await connected({ copy: false, paste: true });
    selected(socket, screen, 'needle');
    const { event, setData } = clipboardEvent(type);
    field.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(setData).not.toHaveBeenCalled();
  });

  it.each([
    ['on', ON, 'needle', false],
    ['off', { copy: false, paste: true }, '', true],
  ])('copy %s: the context menu puts the selection in the text field only when copy is on', async (_, transfer, value, prevented) => {
    const { socket, screen, field, target } = await connected(transfer);
    selected(socket, screen, 'needle');
    const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    target.querySelector('.xterm-screen')?.dispatchEvent(event);
    expect(field.value).toBe(value);
    expect(event.defaultPrevented).toBe(prevented);
  });
});

describe('copy with no open session (RA-033)', () => {
  it('a copy-on device copies nothing once its session ended, nor while the next one starts', async () => {
    const { socket, screen, field, target } = await connected(ON);
    selected(socket, screen, 'needle');
    socket.closeWith(1006);
    await settle();
    const ended = clipboardEvent('copy');
    field.dispatchEvent(ended.event);
    key(field, { ...C, ctrlKey: true, shiftKey: true });
    await settle();
    expect(ended.event.defaultPrevented).toBe(true);
    expect(ended.setData).not.toHaveBeenCalled();
    expect(systemClipboard.written).toEqual([]);
    // Reconnect has asked for a new ticket and the hub has not answered yet.
    target.querySelector<HTMLButtonElement>('.panel button')?.click();
    const starting = clipboardEvent('copy');
    field.dispatchEvent(starting.event);
    expect(starting.event.defaultPrevented).toBe(true);
  });
});

describe('paste into the terminal (RA-033, K4 §8.7)', () => {
  it.each([
    ['on', ON, '\x1b[200~ls\x1b[201~'],
    ['off', { copy: true, paste: false }, ''],
  ])('paste %s: a paste event (shortcut or menu) reaches the shell, bracketed, only when paste is on', async (_, transfer, expected) => {
    const { socket, field } = await connected(transfer);
    socket.output('\x1b[?2004h');
    vi.advanceTimersByTime(100);
    const { event } = clipboardEvent('paste', 'ls');
    field.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(sent(socket)).toBe(expected);
  });

  it('paste on: the Keys menu "Paste" reads the clipboard and sends it', async () => {
    const { socket, target } = await connected(ON);
    systemClipboard.text = 'ls';
    openKeysMenu(target)?.click();
    await settle();
    expect(systemClipboard.reads).toBe(1);
    expect(sent(socket)).toBe('ls');
  });

  it('paste off: the Keys menu "Paste" shows why it is off, and a press reads no clipboard and sends nothing', async () => {
    const { socket, target } = await connected({ copy: true, paste: false });
    systemClipboard.text = 'ls';
    const item = openKeysMenu(target);
    expect(item?.getAttribute('aria-disabled')).toBe('true');
    expect(item?.querySelector('.hint')?.textContent).toBe('Off for this device');
    item?.click();
    await settle();
    expect(systemClipboard.reads).toBe(0);
    expect(socket.typed()).toEqual([]);
  });

  it('paste off: the screen pastes nothing, whoever asks it to', async () => {
    const { socket, screen } = await connected({ copy: true, paste: false });
    screen.paste('ls');
    expect(socket.typed()).toEqual([]);
  });

  it.each([
    ['on', ON, 'safe[201~ls'],
    ['off', { copy: true, paste: false }, ''],
  ])('paste %s: dropped text is pasted cleaned only when paste is on, and never inserted by the browser', async (_, transfer, expected) => {
    const { socket, field } = await connected(transfer);
    const event = dropEvent('safe\x1b[201~ls');
    field.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(sent(socket)).toBe(expected);
  });
});

describe('typing and other edits of the text field (RA-003, RA-033)', () => {
  it.each(['insertFromPaste', 'insertFromPasteAsQuotation', 'insertFromDrop', 'insertFromYank'])('an edit of type %s cannot put outside text into the text field', async (inputType) => {
    const { field } = await connected(ON);
    const event = new InputEvent('beforeinput', { inputType, data: 'ls', bubbles: true, cancelable: true });
    field.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it('paste off: typed keys still reach the shell', async () => {
    const { socket, field } = await connected(OFF);
    key(field, { key: 'a', code: 'KeyA', keyCode: 65 });
    const typing = new InputEvent('beforeinput', { inputType: 'insertText', data: 'a', bubbles: true, cancelable: true });
    field.dispatchEvent(typing);
    expect(typing.defaultPrevented).toBe(false);
    expect(sent(socket)).toBe('a');
  });
});

describe('the policy the view shows (RA-033)', () => {
  it.each([
    [ON, undefined],
    [{ copy: false, paste: true }, 'Copy from the device is off for this device.'],
    [{ copy: true, paste: false }, 'Paste to the device is off for this device.'],
    [OFF, 'Copy and paste are off for this device.'],
  ])('%o: the toolbar says what is off, with the reason as its tooltip', async (transfer, notice) => {
    const { target } = await connected(transfer);
    const shown = target.querySelector('.toolbar > [data-policy]');
    expect(shown?.textContent).toBe(notice);
    expect(shown?.getAttribute('title') ?? undefined).toBe(notice);
  });

  it('an unread policy turns copy and paste off, and one notice says why', async () => {
    const { socket, screen, field, target } = await connected('unanswered');
    expect(target.querySelectorAll('[data-policy]')).toHaveLength(1);
    expect(target.querySelector('.toolbar > [data-policy]')?.textContent).toBe(UNREAD_NOTICE);
    expect(openKeysMenu(target)?.querySelector('.hint')?.textContent).toBe('Off: the hub did not answer');
    selected(socket, screen, 'needle');
    key(field, { ...C, ctrlKey: true, shiftKey: true });
    field.dispatchEvent(clipboardEvent('paste', 'ls').event);
    await settle();
    expect(systemClipboard.written).toEqual([]);
    expect(socket.typed()).toEqual([]);
  });

  it('Reconnect reads the policy again, so a policy changed since the last start applies', async () => {
    const { socket, answers, field, target } = await connected(ON);
    field.dispatchEvent(clipboardEvent('paste', 'ls').event);
    expect(sent(socket)).toBe('ls');
    socket.closeWith(1006);
    await settle();
    answers.transfer = { copy: true, paste: false };
    target.querySelector<HTMLButtonElement>('.panel button')?.click();
    await settle();
    const next = FakeTerminalSocket.latest();
    next.open();
    flushSync();
    textareaOf(target).dispatchEvent(clipboardEvent('paste', 'ls').event);
    expect(next).not.toBe(socket);
    expect(field).toBe(textareaOf(target));
    expect(next.typed()).toEqual([]);
    expect(target.querySelector('[data-policy]')?.textContent).toBe('Paste to the device is off for this device.');
  });
});
