// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// xterm.js reads the platform once, when it loads: as Firefox on Linux it puts a mouse selection in its text field
// (the primary selection) and handles a right click on `mousedown`. Node's `process.title` would make it think it
// runs in Node, so it is hidden until xterm.js has loaded.
const hidden = vi.hoisted(() => {
  Object.defineProperty(navigator, 'platform', { configurable: true, value: 'Linux x86_64' });
  Object.defineProperty(navigator, 'userAgent', { configurable: true, value: 'Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0' });
  const title = Object.getOwnPropertyDescriptor(process, 'title');
  Reflect.deleteProperty(process, 'title');
  return title;
});

import type { SessionTransfer } from '../../api';
import type { TerminalScreen } from '../../screen/screen';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTerminalSocket } from '../../tunnel/terminal/fake/socket.fixture';
import { connectView, flags, installBrowserFakes, showView, textareaOf } from '../view.fixture.svelte';

if (hidden !== undefined) Object.defineProperty(process, 'title', hidden);

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

const COPY_OFF: SessionTransfer = { copy: false, paste: true };
const ON: SessionTransfer = { copy: true, paste: true };

/** A connected SSH view of a device answering `transfer`, with "needle" drawn and selected. */
async function selectedNeedle(transfer: SessionTransfer) {
  let screen: TerminalScreen | undefined;
  const view = showView((open) => async (host, hooks) => (screen = await open(host, hooks)));
  view.answers.transfer = transfer;
  cleanups.push(view.close);
  const socket = await connectView();
  socket.output('needle');
  vi.advanceTimersByTime(100);
  expect(screen?.find('needle', false, false)).toBe(true);
  const surface = view.target.querySelector('.xterm-screen');
  if (surface === null) throw new Error('no terminal screen');
  return { field: textareaOf(view.target), surface };
}

describe('the primary selection on Linux (RA-033, K4 §8.7)', () => {
  it.each([
    ['on', ON, 'needle'],
    ['off', COPY_OFF, ''],
  ])('copy %s: a mouse selection reaches the text field (and so the primary selection) only when copy is on', async (_, transfer, value) => {
    const { field, surface } = await selectedNeedle(transfer);
    surface.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0, detail: 1, shiftKey: true }));
    document.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true, button: 0 }));
    expect(field.value).toBe(value);
  });

  it.each([
    ['on', ON, 'needle'],
    ['off', COPY_OFF, ''],
  ])('copy %s: a right click (handled on mousedown) puts the selection in the text field only when copy is on', async (_, transfer, value) => {
    const { field, surface } = await selectedNeedle(transfer);
    surface.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 2 }));
    expect(field.value).toBe(value);
  });
});
