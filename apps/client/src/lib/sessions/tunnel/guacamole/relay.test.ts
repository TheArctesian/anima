// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../fake/canvas.fixture';
import { codePoints, encodeInstruction, FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from '../fake/socket.fixture';
import { openTunnel, type SessionTunnel } from './tunnel';

// The hub relays gateway instructions unchanged, with element lengths in code points (K4 §4, GW-014, R-046).
// 1.5.0 counted UTF-16 units, so each non-BMP character in an element ended the element one unit early and the
// rest of the text was parsed as instructions of its own. These probes fail on 1.5.0 and pass on 1.5.5.

/** Text that, cut short by one character, reads as a forged clipboard stream and a forged gateway error. */
const FORGED = 'x;9.clipboard,1.0,10.text/plain;4.blob,1.0,8.Zm9yZ2Vk;3.end,1.0;5.error,13.forged detail,3.769';

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

/** An open tunnel whose client records every `name`, `clipboard` and `error` it hears. */
function listening() {
  const opening = openTunnel({
    origin: 'https://hub.anima.ing',
    ticket: { tunnelPath: '/tunnel', token: 't'.repeat(43) },
    WebSocket: FAKE_TUNNEL_SOCKET,
  });
  if (!opening.ok) throw new Error(opening.error.message);
  const tunnel = opening.value;
  tunnels.push(tunnel);
  const socket = FakeTunnelSocket.latest();
  socket.open();
  const heard = { names: [] as string[], clipboards: [] as string[] };
  tunnel.client.onname = (name) => heard.names.push(name);
  tunnel.client.onclipboard = (_stream, mimetype) => heard.clipboards.push(mimetype);
  return { tunnel, socket, heard };
}

describe('relay length counting (K4 §4, GW-014, R-035, R-046)', () => {
  it('R-046: a name of k emoji then forged instructions arrives whole, and nothing forged runs', () => {
    const { tunnel, socket, heard } = listening();
    // k is chosen so that a UTF-16 count stops right after "x", where the forged text starts.
    const name = '😀'.repeat(codePoints(FORGED) - 1) + FORGED;
    socket.receive(encodeInstruction('name', name));
    expect(heard.names).toEqual([name]);
    expect(heard.clipboards).toEqual([]);
    expect(tunnel.state()).toMatchObject({ gatewayError: undefined, phase: 'waiting' });
  });

  it('R-046: the probe text with no emoji is one plain name too', () => {
    const { tunnel, socket, heard } = listening();
    socket.instruction('name', FORGED);
    expect(heard.names).toEqual([FORGED]);
    expect(heard.clipboards).toEqual([]);
    expect(tunnel.state().gatewayError).toBeUndefined();
  });

  for (const name of ['nas-01 (é, 中文, ;,.)', '😀 host 🚀 𝔸𝔹', '😀', 'a😀b😀c', '']) {
    it(`K4 §4: the name ${JSON.stringify(name)} round-trips, alone and between other instructions`, () => {
      const { socket, heard } = listening();
      socket.receive(encodeInstruction('name', name));
      socket.receive(encodeInstruction('nop') + encodeInstruction('name', name) + encodeInstruction('name', 'after'));
      expect(heard.names).toEqual([name, name, 'after']);
    });
  }

  it('R-046: the client counts its own lengths in code points', () => {
    const { tunnel, socket } = listening();
    socket.sync();
    tunnel.client.createClipboardStream('text/😀');
    const sent = socket.sent.filter((message) => message.includes('clipboard'));
    expect(sent).toEqual([encodeInstruction('clipboard', 0, 'text/😀')]);
  });
});
