// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from './fake/canvas.fixture';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from './fake/socket.fixture';
import { sendArgument } from './argv';
import { openTunnel, type SessionTunnel } from './guacamole/tunnel';

let restoreCanvas: () => void = () => undefined;
const tunnels: SessionTunnel[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  restoreCanvas = installBlankCanvas();
});

afterEach(() => {
  for (const tunnel of tunnels.splice(0)) tunnel.close();
  restoreCanvas();
  vi.useRealTimers();
});

/** A connected tunnel and its socket. */
function connected() {
  const opening = openTunnel({ origin: 'https://hub.anima.ing', ticket: { tunnelPath: '/tunnel', token: 't'.repeat(43) }, waking: false, WebSocket: FAKE_TUNNEL_SOCKET });
  if (!opening.ok) throw new Error(opening.error.message);
  tunnels.push(opening.value);
  const socket = FakeTunnelSocket.latest();
  socket.open();
  socket.sync();
  return { client: opening.value.client, socket };
}

/** The `argv`, `blob` and `end` instructions the client sent, the blobs decoded as UTF-8. */
function streamed(socket: FakeTunnelSocket) {
  return socket
    .sentInstructions()
    .filter(([opcode]) => opcode === 'argv' || opcode === 'blob' || opcode === 'end')
    .map(([opcode, ...args]) => (opcode === 'blob' ? [opcode, args[0], Buffer.from(args[1] ?? '', 'base64').toString('utf8')] : [opcode, ...args]));
}

describe('argument value stream (GW-015, K4 §4)', () => {
  it('GW-015: sends argv with the name and text/plain, the value as UTF-8, then end, on one stream', () => {
    const { client, socket } = connected();
    sendArgument(client, 'password', 'pä😀ss');
    expect(streamed(socket)).toEqual([
      ['argv', '0', 'text/plain', 'password'],
      ['blob', '0', 'pä😀ss'],
      ['end', '0'],
    ]);
  });

  it('a one-character value is one blob', () => {
    const { client, socket } = connected();
    sendArgument(client, 'username', 'a');
    expect(streamed(socket)).toEqual([
      ['argv', '0', 'text/plain', 'username'],
      ['blob', '0', 'a'],
      ['end', '0'],
    ]);
  });

  it('an empty value is an argv stream with no blob', () => {
    const { client, socket } = connected();
    sendArgument(client, 'domain', '');
    expect(streamed(socket)).toEqual([
      ['argv', '0', 'text/plain', 'domain'],
      ['end', '0'],
    ]);
  });
});
