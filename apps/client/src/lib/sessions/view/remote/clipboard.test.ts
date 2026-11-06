// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import type { SessionTransfer } from '../../api';
import { UNREAD_POLICY } from '../../transfer';
import { FakeResizeObserver, installBrowserFakes, settle, systemClipboard } from '../view.fixture.svelte';
import { clipboardRules } from './clipboard';
import { chooseKey, connectRemote, keyOn, keysOf, pasteOn, remoteFlags, remotePaneOf, sentOf, showRemote } from './remote.fixture.svelte';

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  remoteFlags.visible = true;
  remoteFlags.focused = true;
  cleanups = [installBlankCanvas(), installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
  vi.useRealTimers();
});

/** A connected view whose device has `transfer` as its policy. */
async function connected(transfer: SessionTransfer | 'unread') {
  const view = showRemote();
  cleanups.push(view.close);
  view.answers.transfer = transfer;
  const socket = await connectRemote();
  return { ...view, socket, pane: remotePaneOf(view.target) };
}

const pasteItem = () => [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((item) => item.querySelector('.label')?.textContent === 'Paste');
const policyText = (target: HTMLElement) => target.querySelector('[data-policy]')?.textContent;
/** The text of every clipboard stream the client sent, decoded from its blobs. */
const pasted = (socket: FakeTunnelSocket) => sentOf(socket, 'blob').map(([, data]) => new TextDecoder().decode(Uint8Array.from(atob(data ?? ''), (c) => c.charCodeAt(0))));
const CTRL_V: [number, number][] = [
  [1, 0xffe3],
  [1, 0x76],
  [0, 0x76],
  [0, 0xffe3],
];

describe('RA-005, RA-033: device to system (copy)', () => {
  it('RA-005: with copy on, text the device copies reaches the system clipboard', async () => {
    const { socket } = await connected({ copy: true, paste: false });
    socket.clipboard('from the desk');
    await settle();
    expect(systemClipboard.written).toEqual(['from the desk']);
  });

  it('RA-033: with copy off, nothing the device copies reaches the system clipboard', async () => {
    const write = vi.spyOn(systemClipboard, 'writeText');
    const { socket, target } = await connected({ copy: false, paste: true });
    socket.clipboard('secret');
    await settle();
    expect(write).not.toHaveBeenCalled();
    expect(policyText(target)).toBe('Copy from the device is off for this device.');
  });

  it('D-8: a hidden session never writes the system clipboard', async () => {
    const { socket } = await connected({ copy: true, paste: true });
    remoteFlags.visible = false;
    flushSync();
    socket.clipboard('background');
    await settle();
    expect(systemClipboard.written).toEqual([]);
  });

  it('a refused clipboard write shows a fixed notice, never the text', async () => {
    const { socket, target } = await connected({ copy: true, paste: true });
    systemClipboard.refuse = true;
    socket.clipboard('CLIP-MARKER');
    await settle();
    expect(target.textContent).toContain('Could not copy the text to your clipboard.');
    expect(target.textContent).not.toContain('CLIP-MARKER');
  });
});

describe('RA-005, RA-033: system to device (paste)', () => {
  it('RA-005: with paste on, "Paste" sends the system clipboard as one stream, then Ctrl+V', async () => {
    const { socket } = await connected({ copy: false, paste: true });
    systemClipboard.text = 'to the desk';
    chooseKey(document.body, 'Paste');
    await settle();
    expect(sentOf(socket, 'clipboard').map(([, type]) => type)).toEqual(['text/plain']);
    expect(pasted(socket)).toEqual(['to the desk']);
    expect(keysOf(socket)).toEqual(CTRL_V);
  });

  it('RA-005: with paste on, the operator\'s paste shortcut sends the pasted text and keeps a held Ctrl down', async () => {
    const { socket, pane } = await connected({ copy: false, paste: true });
    keyOn(pane, 'keydown', { key: 'Control', code: 'ControlLeft', keyCode: 17, ctrlKey: true });
    keyOn(pane, 'keydown', { key: 'v', code: 'KeyV', keyCode: 86, ctrlKey: true });
    const event = pasteOn(pane, 'shortcut text');
    expect(event.defaultPrevented).toBe(true);
    expect(pasted(socket)).toEqual(['shortcut text']);
    expect(keysOf(socket)).toEqual([
      [1, 0xffe3],
      [1, 0x76],
      [0, 0x76],
    ]);
  });

});

describe('RA-033: paste off (system to device)', () => {
  it('RA-033: with paste off, "Paste" is off with its reason, and neither it nor the shortcut sends anything', async () => {
    const read = vi.spyOn(systemClipboard, 'readText');
    const { socket, pane, target } = await connected({ copy: true, paste: false });
    target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
    flushSync();
    const item = pasteItem();
    expect(item?.getAttribute('aria-disabled')).toBe('true');
    expect(item?.textContent).toContain('Off for this device');
    item?.click();
    const event = pasteOn(pane, 'not sent');
    await settle();
    expect(event.defaultPrevented).toBe(true);
    expect(read).not.toHaveBeenCalled();
    expect(sentOf(socket, 'clipboard')).toEqual([]);
    expect(target.textContent).toContain('Paste to the device is off, so nothing was sent.');
    expect(policyText(target)).toBe('Paste to the device is off for this device.');
  });

  it('RA-033: with paste off, Ctrl+V reaches the device, which pastes its own clipboard', async () => {
    const { socket, pane } = await connected({ copy: true, paste: false });
    keyOn(pane, 'keydown', { key: 'Control', code: 'ControlLeft', keyCode: 17, ctrlKey: true });
    keyOn(pane, 'keydown', { key: 'v', code: 'KeyV', keyCode: 86, ctrlKey: true });
    expect(keysOf(socket)).toEqual([
      [1, 0xffe3],
      [1, 0x76],
    ]);
  });

});

describe('RA-005: pastes that must not reach the device', () => {
  it('RA-005: a paste before the first frame (the tunnel open, not yet connected) sends nothing', async () => {
    const view = showRemote();
    cleanups.push(view.close);
    FakeResizeObserver.resize(800, 600);
    await settle();
    const socket = FakeTunnelSocket.latest();
    socket.open();
    flushSync();
    const event = pasteOn(remotePaneOf(view.target), 'too early');
    expect(event.defaultPrevented).toBe(false);
    expect(sentOf(socket, 'clipboard')).toEqual([]);
    expect(sentOf(socket, 'key')).toEqual([]);
  });

  it('RA-005: a clipboard read still pending at a Reconnect never reaches the new tunnel', async () => {
    const { socket, target } = await connected({ copy: true, paste: true });
    let answer: (text: string) => void = () => undefined;
    vi.spyOn(systemClipboard, 'readText').mockImplementationOnce(() => new Promise((resolve) => (answer = resolve)));
    chooseKey(target, 'Paste');
    socket.closeWith(1000);
    await settle();
    [...target.querySelectorAll('button')].find((button) => button.textContent.includes('Reconnect'))?.click();
    const fresh = await connectRemote();
    expect(fresh).not.toBe(socket);
    answer('stale paste');
    await settle();
    expect(sentOf(fresh, 'clipboard')).toEqual([]);
    expect(sentOf(fresh, 'key')).toEqual([]);
    expect(sentOf(socket, 'clipboard')).toEqual([]);
  });

  it('a refused clipboard read shows a fixed notice and sends nothing', async () => {
    const { socket, target } = await connected({ copy: true, paste: true });
    systemClipboard.refuse = true;
    chooseKey(target, 'Paste');
    await settle();
    expect(sentOf(socket, 'clipboard')).toEqual([]);
    expect(target.textContent).toContain('Could not read your clipboard.');
  });
});

describe('D-3: a policy that was not read', () => {
  it('D-3: copy and paste are both off, with one notice saying why', async () => {
    const { socket, pane, target } = await connected('unread');
    socket.clipboard('secret');
    pasteOn(pane, 'not sent');
    chooseKey(target, 'Paste');
    await settle();
    expect(systemClipboard.written).toEqual([]);
    expect(sentOf(socket, 'clipboard')).toEqual([]);
    expect(target.querySelectorAll('[data-policy]')).toHaveLength(1);
    expect(policyText(target)).toBe('Copy and paste are off: the hub did not say whether this device allows them.');
  });

  it('with both on, no policy notice is shown', async () => {
    const { target } = await connected({ copy: true, paste: true });
    expect(target.querySelector('[data-policy]')).toBeNull();
  });
});

describe('clipboardRules (RA-033, D-3)', () => {
  it.each([
    [{ copy: true, paste: true, read: true }, true, true],
    [{ copy: true, paste: false, read: true }, true, false],
    [{ copy: false, paste: true, read: true }, false, true],
    [{ copy: false, paste: false, read: true }, false, false],
    [{ copy: true, paste: true, read: false }, false, false],
    [UNREAD_POLICY, false, false],
  ])('%o gives copy %s and paste %s', (policy, copy, paste) => {
    const rules = clipboardRules(policy);
    expect([rules.copy, rules.paste]).toEqual([copy, paste]);
    expect(rules.pasteOff === undefined).toBe(paste);
    expect(rules.notice === undefined).toBe(copy && paste);
  });

  it('D-3: a flag that is not exactly true counts as off', () => {
    const odd = { copy: 'yes', paste: 1, read: true } as unknown as Parameters<typeof clipboardRules>[0];
    expect(clipboardRules(odd)).toMatchObject({ copy: false, paste: false });
  });
});
