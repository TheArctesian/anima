// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import { paneKeyboard } from '../../tunnel/guacamole/input';
import { openTunnel, type SessionTunnel } from '../../tunnel/guacamole/tunnel';
import { followPaneFocus, keyRoute, pasteText, sendCombination, showDisplay } from './input';
import { KEY_COMBINATIONS, PASTE_STROKES, keyFilter } from './keys';

function fakeClient() {
  const sendKeyEvent = vi.fn<(pressed: 0 | 1, keysym: number) => void>();
  return { sendKeyEvent, typed: { sendKeyEvent } as unknown as SessionTunnel['client'] };
}

describe('Guacamole key routing (RA-006, D-8)', () => {
  it('D-8: a key the filter passes is sent as a press and then a release', () => {
    const { sendKeyEvent, typed } = fakeClient();
    const route = keyRoute(typed, keyFilter({ paste: true }));
    expect(route.down(0x61)).toBe(false);
    route.up(0x61);
    expect(sendKeyEvent.mock.calls).toEqual([
      [1, 0x61],
      [0, 0x61],
    ]);
  });

  it('a key held with Command stays with the browser, and its release is not sent', () => {
    const { sendKeyEvent, typed } = fakeClient();
    const route = keyRoute(typed, keyFilter({ paste: true }));
    expect(route.down(0xffe7)).toBe(true);
    expect(route.down(0x76)).toBe(true);
    route.up(0x76);
    expect(sendKeyEvent).not.toHaveBeenCalled();
  });

  it('RA-006: Ctrl+Alt+Delete is sent as its presses in order and its releases in reverse', () => {
    const { sendKeyEvent, typed } = fakeClient();
    const combination = KEY_COMBINATIONS.find((entry) => entry.id === 'ctrl-alt-delete');
    if (combination === undefined) throw new Error('no Ctrl+Alt+Delete');
    sendCombination(typed, combination);
    expect(sendKeyEvent.mock.calls).toEqual([
      [1, 0xffe3],
      [1, 0xffe9],
      [1, 0xffff],
      [0, 0xffff],
      [0, 0xffe9],
      [0, 0xffe3],
    ]);
  });
});

let restoreCanvas: () => void = () => undefined;
beforeEach(() => {
  FakeTunnelSocket.reset();
  restoreCanvas = installBlankCanvas();
});
afterEach(() => {
  restoreCanvas();
  document.body.innerHTML = '';
});

function connected() {
  const opening = openTunnel({ origin: 'https://hub.anima.ing', ticket: { tunnelPath: '/tunnel', token: 'x'.repeat(40) }, waking: false, WebSocket: FAKE_TUNNEL_SOCKET });
  if (!opening.ok) throw new Error('not opened');
  const socket = FakeTunnelSocket.latest();
  socket.open();
  socket.sync();
  const sent = (opcode: string) => socket.sentInstructions().filter(([name]) => name === opcode).map(([, ...args]) => args);
  return { tunnel: opening.value, socket, sent };
}

function press(target: EventTarget, type: 'keydown' | 'keyup', init: KeyboardEventInit & { keyCode: number }) {
  const event = new KeyboardEvent(type, { bubbles: true, cancelable: true, ...init });
  Object.defineProperty(event, 'keyCode', { value: init.keyCode });
  target.dispatchEvent(event);
}
const enter = { key: 'Enter', code: 'Enter', keyCode: 13 };

describe('the Guacamole pointer and keyboard (D-8)', () => {
  it('D-8: the pane keyboard hears its own element only, and stops after unbind, releasing a held key once', () => {
    const { tunnel, sent } = connected();
    const pane = document.createElement('div');
    document.body.append(pane);
    const keyboard = paneKeyboard(pane);
    keyboard.bind(keyRoute(tunnel.client, keyFilter({ paste: true })));
    press(document.body, 'keydown', enter);
    expect(sent('key')).toEqual([]);
    press(pane, 'keydown', enter);
    keyboard.unbind();
    press(pane, 'keyup', enter);
    expect(sent('key')).toEqual([
      ['65293', '1'],
      ['65293', '0'],
    ]);
  });

  it('the wheel on the display sends scroll button presses', () => {
    const { tunnel, sent } = connected();
    const host = document.createElement('div');
    document.body.append(host);
    const stop = showDisplay(tunnel.client)(host);
    const element = host.firstElementChild;
    element?.dispatchEvent(new WheelEvent('wheel', { deltaY: 120, deltaMode: 0, bubbles: true, cancelable: true }));
    const masks = sent('mouse').map(([, , mask]) => Number(mask));
    expect(masks).toContain(16);
    expect(masks.at(-1)).toBe(0);
    if (typeof stop === 'function') stop();
    expect(host.firstElementChild).toBeNull();
  });
});

describe('forwardPointer (D-8)', () => {
  function mouse(target: Element, type: string, x: number, y: number) {
    target.dispatchEvent(new MouseEvent(type, { clientX: x, clientY: y, bubbles: true, cancelable: true }));
  }

  it('D-8: a pointer position is sent in remote pixels, undoing the display scale', () => {
    const { tunnel, sent } = connected();
    const host = document.createElement('div');
    document.body.append(host);
    tunnel.client.getDisplay().scale(0.5);
    const stop = showDisplay(tunnel.client)(host);
    const element = host.firstElementChild;
    if (element === null) throw new Error('no display');
    mouse(element, 'mousemove', 100, 50);
    expect(sent('mouse').at(-1)?.slice(0, 2)).toEqual(['200', '100']);
    if (typeof stop === 'function') stop();
  });

  it('D-8: once stopped, the pointer on the display sends nothing', () => {
    const { tunnel, sent } = connected();
    const host = document.createElement('div');
    document.body.append(host);
    const stop = showDisplay(tunnel.client)(host);
    const element = host.firstElementChild;
    if (element === null || typeof stop !== 'function') throw new Error('no display');
    mouse(element, 'mousemove', 10, 10);
    const before = sent('mouse').length;
    expect(before).toBeGreaterThan(0);
    stop();
    mouse(element, 'mousemove', 20, 20);
    mouse(element, 'mousedown', 20, 20);
    expect(sent('mouse')).toHaveLength(before);
  });
});

describe('the Guacamole paste and focus (D-8, RA-005)', () => {
  it('RA-005: pasted text goes as one text/plain clipboard stream, then Ctrl+V; empty text sends nothing', () => {
    const { tunnel, sent } = connected();
    pasteText(tunnel.client, '', new Set());
    expect(sent('clipboard')).toEqual([]);
    pasteText(tunnel.client, 'echo hi', new Set());
    expect(sent('clipboard').map(([, type]) => type)).toEqual(['text/plain']);
    expect(sent('key').map(([keysym, pressed]) => [Number(pressed), Number(keysym)])).toEqual(PASTE_STROKES.map(([pressed, keysym]) => [pressed, keysym]));
  });

  it('RA-005: after the operator\'s Ctrl+V with Ctrl still held, the paste keys press and release only V', () => {
    const { tunnel, sent } = connected();
    pasteText(tunnel.client, 'echo hi', new Set([0xffe3, 0xffe1]));
    expect(sent('key').map(([keysym, pressed]) => [Number(keysym), Number(pressed)])).toEqual([
      [0x76, 1],
      [0x76, 0],
    ]);
  });

  it('D-8: focus is followed into and out of the pane, by focusout, window blur and a hidden page', () => {
    const pane = document.createElement('div');
    const inner = document.createElement('input');
    const outer = document.createElement('input');
    pane.append(inner);
    document.body.append(pane, outer);
    const changes: boolean[] = [];
    const stop = followPaneFocus(pane, (inside) => changes.push(inside));
    inner.focus();
    outer.focus();
    inner.focus();
    window.dispatchEvent(new Event('blur'));
    stop();
    inner.blur();
    inner.focus();
    expect(changes).toEqual([false, true, false, true, false]);
  });
});
