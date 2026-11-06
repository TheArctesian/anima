// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTerminalSocket } from '../../tunnel/terminal/fake/socket.fixture';
import { NEW_KEY, connectView, flags, installBrowserFakes, settle, showView } from '../view.fixture.svelte';

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
  vi.useRealTimers();
});

const dialog = () => document.querySelector<HTMLElement>('[role="alertdialog"]');
const button = (name: string) =>
  [...(dialog()?.querySelectorAll('button') ?? [])].find((candidate) => candidate.textContent.trim() === name);

async function click(name: string) {
  const target = button(name);
  if (target === undefined) throw new Error(`no "${name}" button`);
  target.click();
  await settle();
}

/** A session that ends with a host key change: the `end` message, then 4001, as the hub sends it (K4 §8.3). */
async function hostKeyChanged(closeCode = 4001) {
  const view = showView();
  cleanups.push(view.close);
  const socket = await connectView();
  view.answers.endReason = 'HOST_KEY_MISMATCH';
  socket.end('HOST_KEY_MISMATCH');
  socket.closeWith(closeCode);
  await settle();
  return view;
}

describe('host key modal (SEC-007, D-6)', () => {
  it('SEC-007: close 4001 opens the warning, which names the device and says why it matters', async () => {
    await hostKeyChanged();
    expect(dialog()?.getAttribute('aria-modal')).toBe('true');
    expect(dialog()?.textContent).toContain('web-01');
    expect(dialog()?.textContent).toContain('host key');
    expect(dialog()?.textContent).toContain('passwords');
  });

  it('F-2: a recorded HOST_KEY_MISMATCH opens the warning when the 4001 close is not seen', async () => {
    await hostKeyChanged(1005);
    expect(dialog()).not.toBeNull();
  });

  it('SEC-007: "Close" is the default; it keeps the old key and calls no host key mutation', async () => {
    const { calls } = await hostKeyChanged();
    expect(document.activeElement).toBe(button('Close'));
    await click('Close');
    expect(dialog()).toBeNull();
    expect(calls.testConnection).toEqual([]);
    expect(calls.acceptHostKey).toEqual([]);
  });

});

describe('host key review and accept (SEC-007, D-6)', () => {
  it('SEC-007: no accept is offered before "Review new key" ran testConnection', async () => {
    const { calls } = await hostKeyChanged();
    expect(button('Accept new key')).toBeUndefined();
    await click('Review new key');
    expect(calls.testConnection).toEqual([{ deviceId: 'device-1' }]);
    expect(dialog()?.querySelector('[data-fingerprint]')?.textContent).toBe(NEW_KEY.fingerprintSha256);
  });

  it('SEC-007: "Accept new key" sends exactly the fingerprint shown, once, only after the second confirmation', async () => {
    const { calls } = await hostKeyChanged();
    await click('Review new key');
    const shown = dialog()?.querySelector('[data-fingerprint]')?.textContent;
    await click('Accept new key');
    expect(calls.acceptHostKey).toEqual([]);
    expect(button('Reconnect')).toBeUndefined();
    // A double click: the second click lands while the first accept is under way.
    const confirm = button('Yes, accept this key');
    confirm?.click();
    confirm?.click();
    await settle();
    expect(calls.acceptHostKey).toEqual([['device-1', shown]]);
    expect(button('Reconnect')).toBeDefined();
  });

  it('SEC-007: "Reconnect" after the accept starts a new session in the same pane', async () => {
    const { calls } = await hostKeyChanged();
    await click('Review new key');
    await click('Accept new key');
    await click('Yes, accept this key');
    await click('Reconnect');
    expect(dialog()).toBeNull();
    expect(calls.startSession).toHaveLength(2);
    expect(FakeTerminalSocket.made).toHaveLength(2);
  });

  it('a refused accept shows fixed text and offers no Reconnect', async () => {
    const { answers } = await hostKeyChanged();
    answers.accept = () => ({ ok: false, error: { code: 'CONFLICT', message: 'hub detail' } });
    await click('Review new key');
    await click('Accept new key');
    await click('Yes, accept this key');
    expect(dialog()?.textContent).not.toContain('hub detail');
    expect(button('Reconnect')).toBeUndefined();
    expect(button('Review new key')).toBeDefined();
  });

});

describe('host key modal keyboard (Section 5.7.3 "Modal")', () => {
  it('focus is trapped in the modal and Escape closes it', async () => {
    await hostKeyChanged();
    const first = button('Close');
    const last = button('Review new key');
    last?.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));
    expect(document.activeElement).toBe(first);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true, cancelable: true }));
    expect(document.activeElement).toBe(last);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    flushSync();
    await settle();
    expect(dialog()).toBeNull();
  });

  it('SEC-007, D-8: a hidden session opens its warning only when its tab is shown, never over another tab', async () => {
    const view = showView();
    cleanups.push(view.close);
    const socket = await connectView();
    const other = document.createElement('input');
    document.body.append(other);
    flags.visible = false;
    flushSync();
    other.focus();
    socket.closeWith(4001);
    await settle();
    expect(dialog()).toBeNull();
    expect(document.activeElement).toBe(other);
    flags.visible = true;
    flushSync();
    await settle();
    expect(dialog()).not.toBeNull();
  });

  it('a session that ends for another reason opens no warning', async () => {
    const view = showView();
    cleanups.push(view.close);
    const socket = await connectView();
    socket.closeWith(1000);
    await settle();
    expect(dialog()).toBeNull();
  });
});
