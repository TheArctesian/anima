import { inspect } from 'node:util';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FAKE_TERMINAL_SOCKET, FakeTerminalSocket } from '../terminal/fake/socket.fixture';
import { PING_AFTER_MS, SILENCE_LIMIT_MS } from '../terminal/link';
import {
  RECORDING_MAX_BYTES,
  openRecordingLink,
  type RecordingLinkOptions,
  type RecordingLinkState,
} from './link';

const TOKEN = 'TOKEN-MARKER-5e7a';
const TICKET = { tunnelPath: '/recording', token: TOKEN };
const HEADER = { type: 'recording', format: 'GUACAMOLE', sizeBytes: 4096, complete: true };
const CONSOLE_METHODS = Object.keys(console).filter(
  (key): key is keyof Console => typeof (console as unknown as Record<string, unknown>)[key] === 'function',
);

function open(extra: Partial<RecordingLinkOptions> = {}) {
  const heard = { bytes: [] as number[][], changes: [] as RecordingLinkState[], ends: [] as RecordingLinkState[] };
  const opening = openRecordingLink({
    origin: 'https://hub.anima.ing',
    ticket: TICKET,
    WebSocket: FAKE_TERMINAL_SOCKET,
    onbytes: (bytes) => heard.bytes.push([...bytes]),
    onchange: (state) => heard.changes.push(state),
    onend: (state) => heard.ends.push(state),
    ...extra,
  });
  if (!opening.ok) throw new Error('not opened');
  return { link: opening.value, socket: FakeTerminalSocket.latest(), heard };
}

/** A link that is open and has its header. */
function receiving(header: unknown = HEADER) {
  const opened = open();
  opened.socket.open();
  opened.socket.control(header);
  return opened;
}

const throwing = function (url: string): never {
  throw new DOMException(`Refused to connect to '${url}'`, 'SecurityError');
} as unknown as typeof WebSocket;

beforeEach(() => {
  vi.useFakeTimers();
  FakeTerminalSocket.reset();
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe('opening (K4 §12, §8.1)', () => {
  it('K4 §12: wss for https, /recording, the token only in the query, subprotocol anima-recording, binary as ArrayBuffer', () => {
    const { socket } = open();
    expect(socket.url).toBe(`wss://hub.anima.ing/recording?token=${TOKEN}`);
    expect(socket.protocol).toBe('anima-recording');
    expect(socket.binaryType).toBe('arraybuffer');
  });

  it('K4 §1: a ticket for another tunnel path, or a clear-text hub, is refused before any socket is made', () => {
    for (const options of [{ ticket: { tunnelPath: '/terminal', token: TOKEN } }, { origin: 'http://hub.example.com' }]) {
      const refused = openRecordingLink({ origin: 'https://hub.anima.ing', ticket: TICKET, WebSocket: FAKE_TERMINAL_SOCKET, onbytes: () => undefined, ...options });
      expect(refused).toMatchObject({ ok: false, error: { problem: 'bad-address' } });
    }
    expect(FakeTerminalSocket.made).toEqual([]);
  });

  it('LOG-007: a socket constructor that throws with the address gives a fixed refusal, logs nothing and holds no token', () => {
    const spies = CONSOLE_METHODS.map((method) => vi.spyOn(console, method).mockImplementation(() => undefined));
    const refused = openRecordingLink({ origin: 'https://hub.anima.ing', ticket: TICKET, WebSocket: throwing, onbytes: () => undefined });
    expect(refused).toMatchObject({ ok: false, error: { problem: 'not-opened' } });
    expect(inspect(refused, { depth: Infinity, showHidden: true })).not.toContain(TOKEN);
    expect(spies.flatMap((spy) => spy.mock.calls)).toEqual([]);
  });

  it('SEC-002, LOG-007: neither the state nor anything reported holds the token, the address or the content', () => {
    const { link, socket, heard } = receiving();
    socket.output('4.name,6.SECRET;');
    socket.control({ type: 'done' });
    const reported = inspect([heard.changes, heard.ends, link.state(), link], { depth: Infinity, showHidden: true });
    expect(reported).not.toContain(TOKEN);
    expect(reported).not.toContain('hub.anima.ing');
    expect(reported).not.toContain('SECRET');
  });

  it('LOG-007: a token holding + / = & reaches the socket URL encoded, so it stays one query value', () => {
    const token = 'a+b/c=d&e';
    openRecordingLink({ origin: 'https://hub.anima.ing', ticket: { tunnelPath: '/recording', token }, WebSocket: FAKE_TERMINAL_SOCKET, onbytes: () => undefined });
    const { url } = FakeTerminalSocket.latest();
    expect(url).toBe('wss://hub.anima.ing/recording?token=a%2Bb%2Fc%3Dd%26e');
    expect(new URL(url).searchParams.get('token')).toBe(token);
  });

  it('K4 §8.1: a socket that closes before it opened ends as refused, with its close code', () => {
    const { socket, heard } = open();
    socket.closeWith(1006);
    expect(heard.ends).toEqual([{ phase: 'ended', header: undefined, receivedBytes: 0, closeCode: 1006, outcome: 'refused' }]);
  });
});

describe('the stream (K4 §12, RA-041)', () => {
  it('K4 §12: the header gives format, size and complete; frames reach the owner in order and are counted', () => {
    const { link, socket, heard } = receiving({ ...HEADER, complete: false });
    expect(link.state()).toMatchObject({ phase: 'receiving', header: { format: 'GUACAMOLE', sizeBytes: 4096, complete: false } });
    socket.output(Uint8Array.from([1, 2]));
    socket.output(Uint8Array.from([3]));
    socket.output(new Uint8Array(65_536));
    expect(heard.bytes.slice(0, 2)).toEqual([[1, 2], [3]]);
    expect(link.state().receivedBytes).toBe(65_539);
  });

  it('RA-041: an incomplete recording\'s sizeBytes bounds nothing; the hub may send past it', () => {
    const { link, socket, heard } = receiving({ ...HEADER, sizeBytes: 3, complete: false });
    socket.output(Uint8Array.from([1, 2, 3, 4]));
    socket.control({ type: 'done' });
    expect(heard.ends).toMatchObject([{ outcome: 'done', receivedBytes: 4 }]);
    expect(link.state().receivedBytes).toBe(4);
  });

});

describe('the end of a playback (K4 §12, RA-041)', () => {
  it.each([
    ['done', { type: 'done' }, 'done'],
    ['failed DAMAGED', { type: 'failed', reason: 'DAMAGED' }, 'damaged'],
    ['failed MISSING', { type: 'failed', reason: 'MISSING' }, 'missing'],
  ])('K4 §12: %s ends in its state at once, closes 1000 without cancel, and is reported once', (_name, message, outcome) => {
    const { socket, heard } = receiving();
    socket.output('x');
    socket.control(message);
    socket.output('late');
    expect(heard.ends).toEqual([{ phase: 'ended', header: expect.anything() as unknown, receivedBytes: 1, closeCode: undefined, outcome }]);
    expect(heard.bytes).toEqual([[0x78]]);
    expect(socket.closedByClient).toBe(1000);
    expect(socket.messages().filter((sent) => sent.type === 'cancel')).toEqual([]);
    return Promise.resolve().then(() => {
      expect(heard.ends).toHaveLength(1);
    });
  });

  it('K4 §12: a missing file may fail before any header', () => {
    const { socket, heard } = open();
    socket.open();
    socket.control({ type: 'failed', reason: 'MISSING' });
    expect(heard.ends.map((state) => state.outcome)).toEqual(['missing']);
  });

  it.each([
    ['binary before the header', [new Uint8Array(1)]],
    ['an unknown format', [{ ...HEADER, format: 'WEBM' }]],
    ['a second header', [HEADER, HEADER]],
    ['a frame over 64 KiB', [{ ...HEADER, complete: false }, new Uint8Array(65_537)]],
    ['a failed with an unknown reason', [{ type: 'failed', reason: 'LOST' }]],
    ['done before the header', [{ type: 'done' }]],
  ])('K4 §12: %s is a protocol fault: the client sends cancel, closes and delivers nothing more', (_name, frames: unknown[]) => {
    const { socket, heard } = open();
    socket.open();
    for (const frame of frames) {
      if (frame instanceof Uint8Array) socket.output(frame);
      else socket.control(frame);
    }
    expect(heard.ends.map((state) => state.outcome)).toEqual(['fault']);
    expect(heard.bytes).toEqual([]);
    expect(socket.messages().at(-1)).toEqual({ type: 'cancel' });
    expect(socket.closedByClient).toBe(1000);
  });

  it('K4 §12: an unknown message type is ignored, as a newer hub may send one', () => {
    const { link, socket } = receiving();
    socket.control({ type: 'progress', parts: 3 });
    socket.control('not json');
    expect(link.state().phase).toBe('receiving');
  });

  it('K4 §12: a close before done ends interrupted with the close code', () => {
    const { socket, heard } = receiving();
    socket.closeWith(1011);
    expect(heard.ends).toMatchObject([{ outcome: 'interrupted', closeCode: 1011 }]);
  });
});

describe('the byte bounds (RA-041, D-14)', () => {
  it('K4 §12: the 64 KiB frame cap alone faults a frame one byte over it; a frame of exactly 64 KiB is accepted', () => {
    const { link, socket, heard } = receiving({ ...HEADER, sizeBytes: 1_000_000, complete: false });
    socket.output(new Uint8Array(65_536));
    expect(heard.bytes.map((frame) => frame.length)).toEqual([65_536]);
    expect(link.state().phase).toBe('receiving');
    socket.output(new Uint8Array(65_537));
    expect(heard.ends.map((state) => state.outcome)).toEqual(['fault']);
    expect(heard.bytes).toHaveLength(1);
  });

  it('RA-041, K4 §12: a complete recording that sends more than its sizeBytes is a fault: the frame past it is not delivered', () => {
    const { link, socket, heard } = receiving({ ...HEADER, sizeBytes: 3, complete: true });
    socket.output(Uint8Array.from([1, 2, 3]));
    socket.output(Uint8Array.from([4]));
    socket.output(Uint8Array.from([5]));
    expect(heard.bytes).toEqual([[1, 2, 3]]);
    expect(heard.ends).toMatchObject([{ outcome: 'fault', receivedBytes: 3 }]);
    expect(link.state().receivedBytes).toBe(3);
    expect(socket.messages().at(-1)).toEqual({ type: 'cancel' });
    expect(socket.closedByClient).toBe(1000);
  });

  it('RA-041, D-14: a frame that would pass maxBytes ends the link as too-large; the bytes before it were delivered', () => {
    const { socket, heard } = open({ maxBytes: 4 });
    socket.open();
    socket.control({ ...HEADER, complete: false });
    socket.output(Uint8Array.from([1, 2, 3]));
    socket.output(Uint8Array.from([4, 5]));
    socket.output(Uint8Array.from([6]));
    expect(heard.bytes).toEqual([[1, 2, 3]]);
    expect(heard.ends).toMatchObject([{ phase: 'ended', outcome: 'too-large', receivedBytes: 3 }]);
    expect(socket.messages().at(-1)).toEqual({ type: 'cancel' });
    expect(socket.closedByClient).toBe(1000);
  });

  it.each([
    ['no maxBytes', {}],
    ['a larger maxBytes', { maxBytes: Number.MAX_SAFE_INTEGER }],
    ['a maxBytes that is not a number', { maxBytes: Number.NaN }],
  ])('D-14: with %s the link takes at most RECORDING_MAX_BYTES (2 GiB, the hub default)', (_name, limits: Partial<RecordingLinkOptions>) => {
    expect(RECORDING_MAX_BYTES).toBe(2 * 1024 ** 3);
    let delivered = 0;
    const { socket, heard } = open({ ...limits, onbytes: (bytes) => (delivered += bytes.byteLength) });
    socket.open();
    socket.control({ ...HEADER, complete: false });
    // One 64 KiB buffer sent again and again: the link only counts it.
    const frame = { data: new ArrayBuffer(65_536) };
    const deliver = socket.onmessage as (event: typeof frame) => void;
    for (let sent = 0; sent < RECORDING_MAX_BYTES / 65_536; sent += 1) deliver(frame);
    expect(heard.ends).toEqual([]);
    deliver(frame);
    expect(delivered).toBe(RECORDING_MAX_BYTES);
    expect(heard.ends).toMatchObject([{ outcome: 'too-large', receivedBytes: RECORDING_MAX_BYTES }]);
  });
});

describe('liveness (K4 §8.4, §8.5)', () => {
  it('K4 §8.4: after PING_AFTER_MS of sending nothing a ping goes, and again after each quiet spell', () => {
    const { socket } = receiving();
    vi.advanceTimersByTime(PING_AFTER_MS - 1);
    expect(socket.messages()).toEqual([]);
    vi.advanceTimersByTime(1);
    socket.output('x');
    vi.advanceTimersByTime(PING_AFTER_MS);
    expect(socket.messages()).toEqual([{ type: 'ping' }, { type: 'ping' }]);
  });

  it('K4 §8.5: SILENCE_LIMIT_MS without a frame ends the link as lost, once, with cancel and a close; any frame starts the silence over', () => {
    const { socket, heard } = receiving();
    vi.advanceTimersByTime(SILENCE_LIMIT_MS - 1);
    socket.control({ type: 'ping' });
    vi.advanceTimersByTime(SILENCE_LIMIT_MS - 1);
    expect(heard.ends).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(heard.ends.map((state) => state.outcome)).toEqual(['lost']);
    expect(socket.messages().at(-1)).toEqual({ type: 'cancel' });
    expect(socket.closedByClient).toBe(1000);
    vi.advanceTimersByTime(SILENCE_LIMIT_MS * 2);
    expect(heard.ends).toHaveLength(1);
  });

});

describe('cancel (K4 §12)', () => {
  it('K4 §12: close() sends cancel, closes 1000, ends as cancelled, stops the pings and drops later frames', () => {
    const { link, socket, heard } = receiving();
    link.close();
    link.close();
    socket.output('late');
    vi.advanceTimersByTime(PING_AFTER_MS * 4);
    expect(socket.messages()).toEqual([{ type: 'cancel' }]);
    expect(socket.closedByClient).toBe(1000);
    expect(heard.bytes).toEqual([]);
    expect(heard.ends.map((state) => state.outcome)).toEqual(['cancelled']);
  });

  it('K4 §12: close() from inside onbytes ends the link once, as cancelled, with nothing reported after the end', () => {
    const changes: RecordingLinkState[] = [];
    const opening = openRecordingLink({
      origin: 'https://hub.anima.ing',
      ticket: TICKET,
      WebSocket: FAKE_TERMINAL_SOCKET,
      onbytes: () => {
        if (opening.ok) opening.value.close();
      },
      onchange: (state) => changes.push(state),
    });
    const socket = FakeTerminalSocket.latest();
    socket.open();
    socket.control(HEADER);
    socket.output('x');
    expect(changes.at(-1)).toMatchObject({ phase: 'ended', outcome: 'cancelled' });
    expect(vi.getTimerCount()).toBe(0);
  });

  it('K4 §1: the link sets no limit on the opening, and close() while connecting sends nothing', () => {
    const { link, socket, heard } = open();
    vi.advanceTimersByTime(120_000);
    expect(heard.ends).toEqual([]);
    link.close();
    expect(socket.frames).toEqual([]);
    expect(heard.ends.map((state) => state.outcome)).toEqual(['cancelled']);
  });
});
