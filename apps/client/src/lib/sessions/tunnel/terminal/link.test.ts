import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { TunnelState } from '../state';
import { FAKE_TERMINAL_SOCKET, FakeTerminalSocket } from './fake/socket.fixture';
import { PING_AFTER_MS, RESIZE_SETTLE_MS, SILENCE_LIMIT_MS, openTerminalLink, type LinkOptions } from './link';

const TOKEN = 'TOKEN-MARKER-9d1c';
const TICKET = { tunnelPath: '/terminal', token: TOKEN };

function open(extra: Partial<LinkOptions> = {}) {
  const heard = { bytes: [] as number[][], changes: [] as TunnelState[], ends: [] as TunnelState[] };
  const opening = openTerminalLink({
    origin: 'https://hub.anima.ing',
    ticket: TICKET,
    WebSocket: FAKE_TERMINAL_SOCKET,
    size: () => ({ cols: 90, rows: 28 }),
    onbytes: (bytes) => heard.bytes.push([...bytes]),
    onchange: (state) => heard.changes.push(state),
    onend: (state) => heard.ends.push(state),
    ...extra,
  });
  if (!opening.ok) throw new Error('not opened');
  return { link: opening.value, socket: FakeTerminalSocket.latest(), heard };
}

beforeEach(() => {
  vi.useFakeTimers();
  FakeTerminalSocket.reset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('opening (K4 §8.1)', () => {
  it('K4 §8.1: wss for https, the ticket path, the token only in the query, subprotocol anima-terminal, binary as ArrayBuffer', () => {
    const { socket } = open();
    expect(socket.url).toBe(`wss://hub.anima.ing/terminal?token=${TOKEN}`);
    expect(socket.protocol).toBe('anima-terminal');
    expect(socket.binaryType).toBe('arraybuffer');
  });

  it('K4 §8.1: ws is allowed for a loopback hub only; a clear-text hub elsewhere is refused before any socket is made', () => {
    open({ origin: 'http://127.0.0.1:8080' });
    expect(FakeTerminalSocket.latest().url).toBe(`ws://127.0.0.1:8080/terminal?token=${TOKEN}`);
    FakeTerminalSocket.reset();
    const refused = openTerminalLink({ origin: 'http://hub.example.com', ticket: TICKET, WebSocket: FAKE_TERMINAL_SOCKET, size: () => ({ cols: 1, rows: 1 }), onbytes: () => undefined });
    expect(refused).toMatchObject({ ok: false, error: { problem: 'bad-address' } });
    expect(FakeTerminalSocket.made).toEqual([]);
  });

  it('LOG-007: a socket constructor that throws with the address in its message gives a fixed refusal without the token', () => {
    const Throwing = function (url: string): never {
      throw new Error(`cannot open ${url}`);
    };
    const refused = openTerminalLink({
      origin: 'https://hub.anima.ing',
      ticket: TICKET,
      WebSocket: Throwing as unknown as typeof WebSocket,
      size: () => ({ cols: 1, rows: 1 }),
      onbytes: () => undefined,
    });
    expect(refused).toMatchObject({ ok: false, error: { problem: 'not-opened' } });
    expect(JSON.stringify(refused)).not.toContain(TOKEN);
  });

  it('SEC-002, LOG-007: neither the state nor anything the link reported holds the token or the address', () => {
    const { socket, heard } = open();
    socket.open();
    socket.end('AUTH_FAILED');
    socket.closeWith(1011);
    const reported = JSON.stringify([heard.changes, heard.ends]);
    expect(reported).not.toContain(TOKEN);
    expect(reported).not.toContain('hub.anima.ing');
  });
});

describe('messages (K4 §8.4)', () => {
  it('K4 §8.4: the first message is resize with the size asked when the socket opened', () => {
    let size = { cols: 90, rows: 28 };
    const { socket } = open({ size: () => size });
    size = { cols: 100, rows: 40 };
    socket.open();
    expect(socket.frames).toEqual(['{"type":"resize","cols":100,"rows":40}']);
  });

  it('K4 §8.4: a size is sent once it settles, and a size the hub already has is not sent again', () => {
    const { link, socket } = open();
    socket.open();
    link.resize({ cols: 91, rows: 28 });
    link.resize({ cols: 92, rows: 28 });
    vi.advanceTimersByTime(RESIZE_SETTLE_MS - 1);
    expect(socket.messages()).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(socket.messages().at(-1)).toEqual({ type: 'resize', cols: 92, rows: 28 });
    link.resize({ cols: 92, rows: 28 });
    vi.advanceTimersByTime(RESIZE_SETTLE_MS);
    expect(socket.messages()).toHaveLength(2);
  });

  it('K4 §8.4: a size set before the socket opens does not hide the first resize', () => {
    const { link, socket } = open();
    link.resize({ cols: 90, rows: 28 });
    vi.advanceTimersByTime(RESIZE_SETTLE_MS);
    socket.open();
    expect(socket.messages()).toEqual([{ type: 'resize', cols: 90, rows: 28 }]);
  });

  it('RA-003: typed bytes are sent as binary frames, unchanged and in order; before the socket opens they are dropped', () => {
    const { link, socket } = open();
    link.send(Uint8Array.from([1]));
    socket.open();
    link.send(Uint8Array.from([0x68, 0x69]));
    link.send(Uint8Array.from([0xff, 0x00]));
    expect(socket.typed()).toEqual([0x68, 0x69, 0xff, 0x00]);
  });

  it('RA-003: bytes from the hub reach the owner unchanged; a text message is not terminal output', () => {
    const { socket, heard } = open();
    socket.open();
    socket.output(Uint8Array.from([0x1b, 0x5b, 0xe2, 0x00]));
    socket.control({ type: 'ping' });
    expect(heard.bytes).toEqual([[0x1b, 0x5b, 0xe2, 0x00]]);
  });

  it('K4 §8.4: the end message is kept as the end reason; one with an unknown reason, a text that is not JSON and an unknown type are ignored', () => {
    const { socket, link } = open();
    socket.open();
    socket.control('not json');
    socket.control({ type: 'future' });
    socket.end('NOT_A_REASON');
    expect(link.state().endReason).toBeUndefined();
    socket.end('HOST_KEY_MISMATCH');
    expect(link.state().endReason).toBe('HOST_KEY_MISMATCH');
  });
});

describe('liveness (K4 §8.4, §8.5)', () => {
  it('K4 §8.4: after PING_AFTER_MS of sending nothing a ping goes; every send delays the next one', () => {
    const { link, socket } = open();
    socket.open();
    vi.advanceTimersByTime(PING_AFTER_MS - 1);
    link.send(Uint8Array.from([1]));
    vi.advanceTimersByTime(PING_AFTER_MS - 1);
    expect(socket.messages().filter((message) => message.type === 'ping')).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(socket.messages().filter((message) => message.type === 'ping')).toEqual([{ type: 'ping' }]);
    vi.advanceTimersByTime(PING_AFTER_MS);
    expect(socket.messages().filter((message) => message.type === 'ping')).toHaveLength(2);
  });

  it('K4 §8.5: SILENCE_LIMIT_MS without a frame from the hub ends the link as lost, once, and closes the socket', () => {
    const { socket, heard } = open();
    socket.open();
    vi.advanceTimersByTime(SILENCE_LIMIT_MS - 1);
    socket.output('x');
    vi.advanceTimersByTime(SILENCE_LIMIT_MS - 1);
    expect(heard.ends).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(heard.ends).toEqual([{ phase: 'disconnected', opened: true, gatewayError: undefined, closeCode: undefined }]);
    expect(socket.closedByClient).toBe(1000);
    vi.advanceTimersByTime(1);
    expect(heard.ends).toHaveLength(1);
  });

  it('K4 §8.5: a ping from the hub counts as a frame', () => {
    const { socket, heard } = open();
    socket.open();
    for (let beat = 0; beat < 10; beat += 1) {
      vi.advanceTimersByTime(SILENCE_LIMIT_MS - 1);
      socket.control({ type: 'ping' });
    }
    expect(heard.ends).toEqual([]);
  });
});

describe('closing', () => {
  function ended(code: number, endReason?: string) {
    const { socket, heard } = open();
    socket.open();
    if (endReason !== undefined) socket.end(endReason);
    socket.closeWith(code);
    return heard.ends;
  }

  it('K4 §8.6: the end state carries the close code and the end reason, and is reported once', () => {
    expect(ended(1011, 'AUTH_FAILED')).toEqual([
      { phase: 'disconnected', opened: true, gatewayError: undefined, closeCode: 1011, endReason: 'AUTH_FAILED' },
    ]);
  });

  it('K4 §8.1: a socket that closes before it opened (a refusal before the upgrade) ends as not opened', () => {
    const { socket, heard } = open();
    socket.closeWith(1006);
    expect(heard.ends).toEqual([{ phase: 'disconnected', opened: false, gatewayError: undefined, closeCode: 1006 }]);
  });

  it('close() closes with 1000, stops the pings and drops later input; the close event still ends the link', () => {
    const { link, socket, heard } = open();
    socket.open();
    link.close();
    link.close();
    expect(socket.closedByClient).toBe(1000);
    vi.advanceTimersByTime(PING_AFTER_MS * 3);
    link.send(Uint8Array.from([1]));
    expect(socket.frames).toHaveLength(1);
    return Promise.resolve().then(() => {
      expect(heard.ends).toHaveLength(1);
    });
  });
});

describe('state', () => {
  it('RA-007: the phase goes connecting, connected, disconnected', () => {
    const { link, socket } = open();
    const phases: TunnelState[] = [link.state()];
    socket.open();
    phases.push(link.state());
    socket.closeWith(1000);
    phases.push(link.state());
    expect(phases.map((state) => state.phase)).toEqual(['connecting', 'connected', 'disconnected']);
    expect(phases[1]?.opened).toBe(true);
  });
});
