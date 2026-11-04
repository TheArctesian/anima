// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ending } from '../ending';
import { installBlankCanvas } from './fake/canvas.fixture';
import { decodeInstructions, encodeInstruction, FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from './fake/socket.fixture';
import { openTunnel, type SessionTunnel, type TunnelState } from './tunnel';
import Guacamole from './vendor/guacamole.js';

/** A ticket token that is easy to find anywhere it should not be (43 base64url characters, K4 §1). */
const TOKEN = 'TOKENMARKER_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const ORIGIN = 'https://hub.anima.ing';

interface Opened {
  readonly tunnel: SessionTunnel;
  readonly socket: FakeTunnelSocket;
  readonly states: TunnelState[];
  readonly ends: TunnelState[];
}

function open(origin = ORIGIN): Opened {
  const states: TunnelState[] = [];
  const ends: TunnelState[] = [];
  const opening = openTunnel({
    origin,
    ticket: { tunnelPath: '/tunnel', token: TOKEN },
    onchange: (state) => states.push(state),
    onend: (state) => ends.push(state),
    WebSocket: FAKE_TUNNEL_SOCKET,
  });
  if (!opening.ok) throw new Error(opening.error.message);
  tunnels.push(opening.value);
  return { tunnel: opening.value, socket: FakeTunnelSocket.latest(), states, ends };
}

const tunnels: SessionTunnel[] = [];
/** Let queued microtasks run: the fake hub answers a client close on one, the tunnel reports its end on the next. */
async function settle(): Promise<void> {
  for (let turn = 0; turn < 3; turn += 1) await Promise.resolve();
}

let restoreCanvas: () => void = () => undefined;

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  restoreCanvas = installBlankCanvas();
});

afterEach(() => {
  for (const tunnel of tunnels.splice(0)) tunnel.close();
  restoreCanvas();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

/** Every string reachable from `root` through own properties (cycles and DOM nodes skipped). */
function strings(root: unknown, seen = new Set<unknown>()): string[] {
  if (typeof root === 'string') return [root];
  if (root === null || typeof root !== 'object' || seen.has(root) || root instanceof Node) return [];
  seen.add(root);
  return Object.values(root).flatMap((value) => strings(value, seen));
}

describe('tunnel request (K4 §2, SEC-002, D-3)', () => {
  it('K4 §2: opens wss://<hub>/tunnel?token=<token> with subprotocol guacamole', () => {
    const { socket } = open();
    expect(socket.url).toBe(`wss://hub.anima.ing/tunnel?token=${TOKEN}`);
    expect(socket.protocol).toBe('guacamole');
  });

  it('D-3: an http hub address gives ws', () => {
    expect(open('http://127.0.0.1:8080').socket.url).toBe(`ws://127.0.0.1:8080/tunnel?token=${TOKEN}`);
  });

  it('GW-013: sends no connect data other than the token and no instruction before the hub opens', () => {
    const { socket } = open();
    expect(new URL(socket.url).searchParams.size).toBe(1);
    expect(socket.sentInstructions()).toEqual([]);
  });

  it('SEC-002: the token is in no state, handle property or DOM node', () => {
    const { tunnel, socket, states } = open();
    socket.open();
    socket.sync();
    document.body.append(tunnel.client.getDisplay().getElement());
    socket.closeWith(1000);
    expect(strings([states, tunnel, tunnel.state()]).filter((text) => text.includes('TOKENMARKER'))).toEqual([]);
    expect(JSON.stringify(states)).not.toContain('TOKENMARKER');
    expect(document.documentElement.outerHTML).not.toContain('TOKENMARKER');
  });

  it('SEC-002: a WebSocket error that names the URL becomes fixed text and the global is restored', () => {
    const real = globalThis.WebSocket;
    // A constructible function, as `new WebSocket` needs, that fails the way a browser does.
    const throwing = function (url: string): never {
      throw new SyntaxError(`The URL '${url}' is invalid.`);
    } as unknown as typeof WebSocket;
    const opening = openTunnel({ origin: ORIGIN, ticket: { tunnelPath: '/tunnel', token: TOKEN }, WebSocket: throwing });
    expect(opening).toEqual({ ok: false, error: { problem: 'not-opened', message: 'The session tunnel could not be opened.' } });
    expect(globalThis.WebSocket).toBe(real);
    // The library's timers were stopped: nothing fires against the missing socket.
    expect(() => vi.advanceTimersByTime(60_000)).not.toThrow();
  });

  it('SEC-002: a tunnel path that names another host is refused before any socket', () => {
    const opening = openTunnel({ origin: ORIGIN, ticket: { tunnelPath: '//evil.example/x', token: TOKEN }, WebSocket: FAKE_TUNNEL_SOCKET });
    expect(opening.ok).toBe(false);
    expect(FakeTunnelSocket.made).toEqual([]);
  });

  it('D-3: the global WebSocket is the real one again after connect', () => {
    const real = globalThis.WebSocket;
    open();
    expect(globalThis.WebSocket).toBe(real);
  });
});

describe('tunnel connect (D-3, K4 §4)', () => {
  it('D-3: onchange is held back while connect runs (the global WebSocket is swapped then)', () => {
    const real = globalThis.WebSocket;
    const heard: { socket: typeof WebSocket; phase: string }[] = [];
    const opening = openTunnel({
      origin: ORIGIN,
      ticket: { tunnelPath: '/tunnel', token: TOKEN },
      WebSocket: FAKE_TUNNEL_SOCKET,
      onchange: (state) => heard.push({ socket: globalThis.WebSocket, phase: state.phase }),
    });
    if (opening.ok) tunnels.push(opening.value);
    FakeTunnelSocket.latest().open();
    // connecting → waiting happened inside connect; it is reported once, after, with the real global back.
    expect(heard.map((entry) => entry.phase)).toEqual(['waiting', 'waiting']);
    expect(heard.every((entry) => entry.socket === real)).toBe(true);
  });

  it('K4 §4: the client pings with the internal instruction and keeps alive with nop', () => {
    const { socket } = open();
    socket.open();
    vi.advanceTimersByTime(5_000);
    // 1.5.5 schedules its `nop` when an instruction arrives and 5 s have passed since the last one.
    socket.sync();
    const internal = socket.sent.flatMap(decodeInstructions).filter(([opcode]) => opcode === '');
    expect(internal.length).toBeGreaterThan(0);
    expect(internal.every(([, name, at]) => name === 'ping' && /^\d+$/.test(at ?? ''))).toBe(true);
    expect(socket.sentInstructions()).toContainEqual(['nop']);
  });

  it('SEC-002: a refused connect reports no change and no end', async () => {
    const changes: TunnelState[] = [];
    const ends: TunnelState[] = [];
    const throwing = function (url: string): never {
      throw new SyntaxError(`The URL '${url}' is invalid.`);
    } as unknown as typeof WebSocket;
    openTunnel({ origin: ORIGIN, ticket: { tunnelPath: '/tunnel', token: TOKEN }, WebSocket: throwing, onchange: (s) => changes.push(s), onend: (s) => ends.push(s) });
    await settle();
    expect(changes).toEqual([]);
    expect(ends).toEqual([]);
  });
});

describe('tunnel state (K4 §4)', () => {

  it('K4 §4: moves through waiting, opened and connected', () => {
    const { tunnel, socket, states } = open();
    expect(tunnel.state()).toMatchObject({ phase: 'waiting', opened: false });
    socket.open();
    expect(tunnel.state()).toMatchObject({ phase: 'waiting', opened: true });
    socket.sync();
    expect(tunnel.state()).toMatchObject({ phase: 'connected', opened: true, closeCode: undefined });
    expect(states.map((state) => state.phase)).toEqual(['waiting', 'waiting', 'connected']);
  });

  it('R-035: a gateway error keeps its status code only, and the client disconnects', async () => {
    const { tunnel, socket, ends } = open();
    socket.open();
    socket.gatewayError(0x0303);
    expect(tunnel.state()).toMatchObject({ phase: 'disconnected', gatewayError: 0x0303 });
    expect(strings(tunnel.state())).not.toContain('Aborted. See logs.');
    expect(socket.sentInstructions()).toContainEqual(['disconnect']);
    await settle();
    expect(ends).toHaveLength(1);
  });

  it('RA-007: a gateway disconnect ends with the code the hub closes with', async () => {
    const { socket, ends } = open();
    socket.open();
    socket.disconnect();
    await settle();
    expect(ends).toEqual([{ phase: 'disconnected', opened: true, gatewayError: undefined, closeCode: 1000 }]);
  });

  it('RA-007: close() sends disconnect, closes the socket once, and is safe to repeat', async () => {
    const { tunnel, socket, ends } = open();
    socket.open();
    tunnel.close();
    tunnel.close();
    await settle();
    expect(socket.sentInstructions().filter(([opcode]) => opcode === 'disconnect')).toHaveLength(1);
    expect(socket.closedByClient).toBe(1000);
    expect(ends).toHaveLength(1);
  });

  it('D-9: gateway clipboard text reaches client.onclipboard through a StringReader', () => {
    const { tunnel, socket } = open();
    socket.open();
    const texts: string[] = [];
    tunnel.client.onclipboard = (stream, mimetype) => {
      expect(mimetype).toBe('text/plain');
      new Guacamole.StringReader(stream).ontext = (text) => texts.push(text);
    };
    socket.clipboard('héllo');
    expect(texts.join('')).toBe('héllo');
  });
});

/** The hub's side of each K4 §5 end, driven on an open tunnel. */
const closes: readonly { readonly name: string; readonly drive: (socket: FakeTunnelSocket) => void; readonly reason: string | undefined }[] = [
  { name: '1000 after disconnect', drive: (s) => { s.disconnect(); s.closeWith(1000); }, reason: 'REMOTE_CLOSED' },
  { name: '1001 hub shutdown', drive: (s) => { s.closeWith(1001); }, reason: 'HUB_SHUTDOWN' },
  { name: '1002 protocol error', drive: (s) => { s.closeWith(1002); }, reason: 'PROTOCOL_ERROR' },
  { name: '1011 after error 0x0300', drive: (s) => { s.gatewayError(0x0300); s.closeWith(1011); }, reason: 'GATEWAY_ERROR' },
  { name: '1011 after error 0x0301', drive: (s) => { s.gatewayError(0x0301); s.closeWith(1011); }, reason: 'AUTH_FAILED' },
  { name: '1011 after error 0x0303', drive: (s) => { s.gatewayError(0x0303); s.closeWith(1011); }, reason: 'AUTH_FAILED' },
  { name: '4001 after error 0x0303', drive: (s) => { s.closeWith(4001); s.gatewayError(0x0303); }, reason: 'HOST_KEY_MISMATCH' },
  { name: '4002 idle timeout', drive: (s) => { s.closeWith(4002); }, reason: 'TIMEOUT' },
];

describe('tunnel end through the real client (K4 §5, RA-007)', () => {
  for (const { name, drive, reason } of closes) {
    it(`K4 §5: ${name} gives ${String(reason)}`, async () => {
      const { socket, ends } = open();
      socket.open();
      drive(socket);
      await settle();
      expect(ends).toHaveLength(1);
      const [end] = ends;
      if (end === undefined) throw new Error('no end');
      expect(ending(end).reason).toBe(reason);
    });
  }

  for (const code of [1002, 1011, 4001, 4002]) {
    it(`K4 §5: close code ${String(code)} is observed, though 1.5.0 reports each of these as 0x0200`, async () => {
      const { socket, ends } = open();
      socket.open();
      socket.closeWith(code);
      await settle();
      expect(ends[0]?.closeCode).toBe(code);
    });
  }

  it('K4 §5: 1006 before open is "Could not open the session"', async () => {
    const { socket, ends } = open();
    socket.closeWith(1006);
    await settle();
    expect(ends[0]).toMatchObject({ opened: false, closeCode: 1006, phase: 'disconnected' });
    expect(ends[0] && ending(ends[0]).text).toBe('Could not open the session.');
  });
});

/** A deterministic generator (CQ-021): the same seed gives the same cases on every run. */
function generator(seed: number) {
  let state = seed;
  return (below: number) => {
    state = (state * 1_103_515_245 + 12_345) % 2 ** 31;
    return state % below;
  };
}

// The hub relays gateway text unchanged, and an element may hold the protocol's own separators. Only BMP
// characters here: 1.5.0 counts element lengths in UTF-16 units, guacd in code points.
const ALPHABET = ['a', 'Z', '0', '9', '.', ',', ';', ' ', 'é', '中', '\\', '"', '\n'];

describe('instruction parsing (K4 §4, property)', () => {
  it('K4 §4: any element text, several instructions per message, reaches the client unchanged', () => {
    const { tunnel, socket } = open();
    socket.open();
    const received: string[] = [];
    tunnel.client.onclipboard = (_stream, mimetype) => received.push(mimetype);
    const next = generator(20_261_104);
    const text = () => Array.from({ length: next(12) }, () => ALPHABET[next(ALPHABET.length)] ?? '').join('');
    const sent: string[] = [];
    for (let message = 0; message < 200; message += 1) {
      const texts = Array.from({ length: 1 + next(3) }, text);
      sent.push(...texts);
      socket.receive(texts.map((mimetype, index) => encodeInstruction('clipboard', message * 3 + index, mimetype)).join(''));
    }
    expect(received).toEqual(sent);
    expect(tunnel.state().phase).toBe('waiting');
  });
});
