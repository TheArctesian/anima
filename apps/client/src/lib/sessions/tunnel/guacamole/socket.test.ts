import { afterEach, describe, expect, it } from 'vitest';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from '../fake/socket.fixture';
import { withObservedSocket } from './socket';

const real = globalThis.WebSocket;

afterEach(() => {
  FakeTunnelSocket.reset();
});

/** `new WebSocket(url)` through whatever the global is at the time, as the library does. */
function construct(url: string): void {
  // The library reads the global at call time; so does this stand-in.
  new globalThis.WebSocket(url, 'guacamole');
}

describe('close code observation (K4 §5)', () => {
  it('K4 §5: the first socket made during the call reports its close code', async () => {
    const codes: number[] = [];
    withObservedSocket(FAKE_TUNNEL_SOCKET, (code) => codes.push(code), () => {
      construct('wss://hub/tunnel');
    });
    FakeTunnelSocket.latest().closeWith(4001);
    await Promise.resolve();
    expect(codes).toEqual([4001]);
    expect(globalThis.WebSocket).toBe(real);
  });

  it('K4 §5: only the first socket is observed', () => {
    const codes: number[] = [];
    withObservedSocket(FAKE_TUNNEL_SOCKET, (code) => codes.push(code), () => {
      construct('wss://hub/one');
      construct('wss://hub/two');
    });
    const [first, second] = FakeTunnelSocket.made;
    second?.closeWith(1002);
    first?.closeWith(1000);
    expect(codes).toEqual([1000]);
  });

  it('SEC-002: when the call throws after the socket was made, the socket is closed, the error passes on and the global is restored', () => {
    const failure = new Error('connect failed');
    expect(() => {
      withObservedSocket(FAKE_TUNNEL_SOCKET, () => undefined, () => {
        construct('wss://hub/tunnel');
        throw failure;
      });
    }).toThrow(failure);
    expect(FakeTunnelSocket.latest().closedByClient).toBe(1000);
    expect(globalThis.WebSocket).toBe(real);
  });
});
