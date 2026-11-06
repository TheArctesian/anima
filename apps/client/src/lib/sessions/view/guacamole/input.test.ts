// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import type { SessionTunnel } from '../../tunnel/guacamole/tunnel';
import { sendCombination, keyRoute } from './input';
import { KEY_COMBINATIONS, keyFilter } from './keys';

function fakeClient() {
  const sendKeyEvent = vi.fn<(pressed: 0 | 1, keysym: number) => void>();
  return { sendKeyEvent, typed: { sendKeyEvent } as unknown as SessionTunnel['client'] };
}

describe('Guacamole key routing (RA-006, D-8; kept for RDP and VNC)', () => {
  it('D-8: a key the filter passes is sent as a press and then a release', () => {
    const { sendKeyEvent, typed } = fakeClient();
    const route = keyRoute(typed, keyFilter());
    expect(route.down(0x61)).toBe(false);
    route.up(0x61);
    expect(sendKeyEvent.mock.calls).toEqual([
      [1, 0x61],
      [0, 0x61],
    ]);
  });

  it('D-9: a key held with Command stays with the browser, and its release is not sent', () => {
    const { sendKeyEvent, typed } = fakeClient();
    const route = keyRoute(typed, keyFilter());
    expect(route.down(0xffe7)).toBe(true);
    expect(route.down(0x76)).toBe(true);
    route.up(0x76);
    expect(sendKeyEvent).not.toHaveBeenCalled();
  });

  it('RA-006: a combination is sent as its presses in order and its releases in reverse', () => {
    const { sendKeyEvent, typed } = fakeClient();
    const ctrlC = KEY_COMBINATIONS.find((entry) => entry.id === 'ctrl-c');
    if (ctrlC === undefined) throw new Error('no Ctrl+C');
    sendCombination(typed, ctrlC);
    expect(sendKeyEvent.mock.calls).toEqual([
      [1, 0xffe3],
      [1, 0x63],
      [0, 0x63],
      [0, 0xffe3],
    ]);
  });
});
