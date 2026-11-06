// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import { FakeResizeObserver, installBrowserFakes, settle } from '../view.fixture.svelte';
import { chooseKey, connectRemote, keyOn, keysOf, remoteFlags, remotePaneOf, sentOf, showRemote } from './remote.fixture.svelte';

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

function show(...args: Parameters<typeof showRemote>) {
  const view = showRemote(...args);
  cleanups.push(view.close);
  return view;
}

const ENTER = { key: 'Enter', code: 'Enter', keyCode: 13 };
const CONTROL = { key: 'Control', code: 'ControlLeft', keyCode: 17, ctrlKey: true };
const ENTER_KEYSYM = 0xff0d;

describe('D-8: the keyboard is bound only while visible, focused and connected', () => {
  it('D-8: a connected, visible and focused pane takes the focus and sends a press and its release', async () => {
    const { target } = show();
    const socket = await connectRemote();
    const pane = remotePaneOf(target);
    expect(document.activeElement).toBe(pane);
    keyOn(pane, 'keydown', ENTER);
    keyOn(pane, 'keyup', ENTER);
    expect(keysOf(socket)).toEqual([
      [1, ENTER_KEYSYM],
      [0, ENTER_KEYSYM],
    ]);
  });

  it('D-8: before the first frame (the gateway may still be asking for a credential) no key is sent', async () => {
    const { target } = show();
    FakeResizeObserver.resize(800, 600);
    await settle();
    const socket = FakeTunnelSocket.latest();
    socket.open();
    flushSync();
    const pane = remotePaneOf(target);
    pane.focus();
    keyOn(pane, 'keydown', ENTER);
    expect(keysOf(socket)).toEqual([]);
  });

  it.each([
    ['hidden', { visible: false, focused: true }],
    ['not focused', { visible: true, focused: false }],
  ])('D-8: a connected pane that is %s sends no key', async (_name, state) => {
    Object.assign(remoteFlags, state);
    const { target } = show();
    const socket = await connectRemote();
    const pane = remotePaneOf(target);
    pane.focus();
    keyOn(pane, 'keydown', ENTER);
    expect(keysOf(socket)).toEqual([]);
  });

});

describe('D-8: held keys are released when the pane loses the keyboard', () => {
  it('D-8: hiding the pane releases a held key on the device at once, and later keys are not sent', async () => {
    const { target } = show();
    const socket = await connectRemote();
    const pane = remotePaneOf(target);
    keyOn(pane, 'keydown', CONTROL);
    remoteFlags.visible = false;
    flushSync();
    keyOn(pane, 'keyup', CONTROL);
    keyOn(pane, 'keydown', ENTER);
    expect(keysOf(socket)).toEqual([
      [1, 0xffe3],
      [0, 0xffe3],
    ]);
  });

  it('D-8: focus leaving the pane releases a held key, and showing it again binds it again', async () => {
    const { target } = show();
    const socket = await connectRemote();
    const pane = remotePaneOf(target);
    const outside = document.createElement('input');
    document.body.append(outside);
    keyOn(pane, 'keydown', CONTROL);
    outside.focus();
    flushSync();
    keyOn(pane, 'keydown', ENTER);
    pane.focus();
    flushSync();
    keyOn(pane, 'keydown', ENTER);
    expect(keysOf(socket)).toEqual([
      [1, 0xffe3],
      [0, 0xffe3],
      [1, ENTER_KEYSYM],
    ]);
  });

});

describe('D-8: the page and the document', () => {
  it('D-8: a hidden page releases a held key and unbinds the keyboard while the focus stays in the pane', async () => {
    const { target } = show();
    const socket = await connectRemote();
    const pane = remotePaneOf(target);
    keyOn(pane, 'keydown', CONTROL);
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
    cleanups.push(() => Reflect.deleteProperty(document, 'visibilityState'));
    document.dispatchEvent(new Event('visibilitychange'));
    flushSync();
    expect(document.activeElement).toBe(pane);
    keyOn(pane, 'keydown', ENTER);
    expect(keysOf(socket)).toEqual([
      [1, 0xffe3],
      [0, 0xffe3],
    ]);
  });

  it('D-8: the keyboard hears the pane only, never the document', async () => {
    show();
    const socket = await connectRemote();
    keyOn(document.body, 'keydown', ENTER);
    expect(keysOf(socket)).toEqual([]);
  });

  it('D-8: a key with Command held stays with the operator\'s system', async () => {
    const { target } = show();
    const socket = await connectRemote();
    const pane = remotePaneOf(target);
    keyOn(pane, 'keydown', { key: 'Meta', code: 'MetaLeft', keyCode: 91, metaKey: true });
    keyOn(pane, 'keydown', { key: 'q', code: 'KeyQ', keyCode: 81, metaKey: true });
    expect(keysOf(socket).filter(([, keysym]) => keysym === 0x71)).toEqual([]);
  });
});

describe('RA-006: the Keys menu', () => {
  it('RA-006: Ctrl+Alt+Delete sends Ctrl, Alt, Delete pressed in order, then released in reverse', async () => {
    const { target } = show();
    const socket = await connectRemote();
    expect(chooseKey(target, 'Ctrl+Alt+Delete')).toBeDefined();
    expect(keysOf(socket)).toEqual([
      [1, 0xffe3],
      [1, 0xffe9],
      [1, 0xffff],
      [0, 0xffff],
      [0, 0xffe9],
      [0, 0xffe3],
    ]);
  });

  it('RA-006: the menu offers no terminal bytes and is off until the session is connected', async () => {
    const { target } = show();
    const trigger = target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
    expect(trigger?.disabled).toBe(true);
    const socket = await connectRemote();
    expect(trigger?.disabled).toBe(false);
    chooseKey(target, 'Windows key');
    expect(keysOf(socket)).toEqual([
      [1, 0xffeb],
      [0, 0xffeb],
    ]);
    expect(sentOf(socket, 'blob')).toEqual([]);
  });
});
