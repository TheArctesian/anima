import { inspect } from 'node:util';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { endText } from './closing';
import { FAKE_FILES_SOCKET, FakeFilesSocket } from './fake/socket.fixture';
import { PING_AFTER_MS, SILENCE_LIMIT_MS, openFilesLink, type FilesLinkOptions, type FilesLinkState } from './link';

const TOKEN = 'TOKEN-MARKER-51fa';
const TICKET = { tunnelPath: '/files', token: TOKEN };
const ENTRIES = { type: 'entries', path: '/home/op', entries: [{ name: 'a.txt', kind: 'file', size: null, modifiedAt: null }] };

function open(extra: Partial<FilesLinkOptions> = {}) {
  const heard = { changes: [] as FilesLinkState[], ends: [] as FilesLinkState[] };
  const opening = openFilesLink({
    origin: 'https://hub.anima.ing',
    ticket: TICKET,
    WebSocket: FAKE_FILES_SOCKET,
    onchange: (state) => heard.changes.push(state),
    onend: (state) => heard.ends.push(state),
    ...extra,
  });
  if (!opening.ok) throw new Error('not opened');
  return { link: opening.value, socket: FakeFilesSocket.latest(), heard };
}

function connected() {
  const opened = open();
  opened.socket.open();
  return opened;
}

beforeEach(() => {
  vi.useFakeTimers();
  FakeFilesSocket.reset();
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe('opening (K4 §9, §8.1)', () => {
  it('K4 §9: wss for https, the ticket path, the token only in the query, subprotocol anima-files, binary as ArrayBuffer', () => {
    const { socket } = open();
    expect(socket.url).toBe(`wss://hub.anima.ing/files?token=${TOKEN}`);
    expect(socket.protocol).toBe('anima-files');
    expect(socket.binaryType).toBe('arraybuffer');
  });

  it('SEC-002: ws only to a loopback hub; a clear-text hub elsewhere is refused before any socket is made', () => {
    open({ origin: 'http://127.0.0.1:8080' });
    expect(FakeFilesSocket.latest().url).toBe(`ws://127.0.0.1:8080/files?token=${TOKEN}`);
    FakeFilesSocket.reset();
    expect(openFilesLink({ origin: 'http://hub.example.com', ticket: TICKET, WebSocket: FAKE_FILES_SOCKET })).toMatchObject({ ok: false, error: { problem: 'bad-address' } });
    expect(FakeFilesSocket.made).toEqual([]);
  });

  it('LOG-007: a constructor that throws with the address gives a fixed refusal, and nothing is logged', () => {
    const spies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((method) => vi.spyOn(console, method).mockImplementation(() => undefined));
    const Throwing = function (url: string): never {
      throw new DOMException(`Refused to connect to '${url}'`, 'SecurityError');
    };
    const refused = openFilesLink({ origin: 'https://hub.anima.ing', ticket: TICKET, WebSocket: Throwing as unknown as typeof WebSocket });
    expect(refused).toMatchObject({ ok: false, error: { problem: 'not-opened' } });
    expect(inspect(refused, { depth: null })).not.toContain(TOKEN);
    for (const spy of spies) expect(spy).not.toHaveBeenCalled();
  });

  it('SEC-002: the link and every state it reports hold no token', async () => {
    const { link, socket, heard } = connected();
    void link.list('/');
    socket.text(ENTRIES);
    socket.closeWith(1011);
    await vi.advanceTimersByTimeAsync(0);
    expect(inspect(link, { depth: null, showHidden: true })).not.toContain(TOKEN);
    expect(JSON.stringify(heard)).not.toContain(TOKEN);
  });
});

describe('liveness (K4 §8.4)', () => {
  it('K4 §8.4: a ping goes out after 5 s of client quiet, and anything sent starts the quiet over', async () => {
    const { link, socket } = connected();
    await vi.advanceTimersByTimeAsync(PING_AFTER_MS - 1);
    expect(socket.types()).toEqual([]);
    void link.list('/');
    socket.text(ENTRIES);
    await vi.advanceTimersByTimeAsync(PING_AFTER_MS - 1);
    expect(socket.types()).toEqual(['list']);
    await vi.advanceTimersByTimeAsync(1);
    expect(socket.types()).toEqual(['list', 'ping']);
  });

  it('K4 §8.4: 15 s without any hub frame is a lost connection; the client closes and the request in flight ends', async () => {
    const { link, socket, heard } = connected();
    const listing = link.list('/');
    await vi.advanceTimersByTimeAsync(SILENCE_LIMIT_MS - 1);
    socket.text({ type: 'ping' });
    await vi.advanceTimersByTimeAsync(SILENCE_LIMIT_MS - 1);
    expect(link.state().phase).toBe('connected');
    await vi.advanceTimersByTimeAsync(1);
    expect(await listing).toEqual({ ok: false, error: { kind: 'ended' } });
    expect(link.state()).toMatchObject({ phase: 'disconnected', end: 'lost' });
    expect(socket.closedByClient).toBe(1000);
    expect(heard.ends).toHaveLength(1);
  });

  it('K4 §8.4: after the link ended no ping goes out', async () => {
    const { link, socket } = connected();
    link.close();
    await vi.advanceTimersByTimeAsync(PING_AFTER_MS * 3);
    expect(socket.frames).toEqual([]);
    expect(link.state()).toMatchObject({ phase: 'disconnected', end: 'closed' });
  });
});

describe('close codes (WP-27 handoff)', () => {
  it.each([
    [1000, 'The file connection closed.'],
    [1001, 'The hub is restarting.'],
    [1002, 'The hub refused a message from this client.'],
    [1011, 'The connection to the device failed.'],
    [4002, 'The connection went quiet.'],
    [4999, 'The file connection ended unexpectedly.'],
  ])('K4 §9: close %i shows fixed text', (code, text) => {
    const { link, socket, heard } = connected();
    socket.closeWith(code);
    expect(link.state()).toMatchObject({ phase: 'disconnected', closeCode: code });
    expect(endText(link.state().end ?? 'ended')).toBe(text);
    expect(heard.ends).toHaveLength(1);
  });

  it('K4 §8.1: a close before the upgrade is a refused opening', () => {
    const { link, socket } = open();
    socket.closeWith(1006);
    expect(link.state()).toMatchObject({ opened: false, end: 'not-opened' });
    expect(endText('not-opened')).toBe('The file connection could not be opened.');
  });
});

describe('list and the closed set (K4 §9)', () => {
  it('K4 §9: list sends the path and settles with the entries the hub sent', async () => {
    const { link, socket } = connected();
    const listing = link.list('/home/op');
    expect(socket.messages()).toEqual([{ type: 'list', path: '/home/op' }]);
    socket.text(ENTRIES);
    expect(await listing).toEqual({ ok: true, value: { path: '/home/op', entries: ENTRIES.entries } });
  });

  it('K4 §9: a refused list settles with the reason, and the link stays open', async () => {
    const { link, socket } = connected();
    const listing = link.list('/root');
    socket.text({ type: 'refused', reason: 'DENIED' });
    expect(await listing).toEqual({ ok: false, error: { kind: 'refused', reason: 'DENIED' } });
    expect(link.state().phase).toBe('connected');
  });

  it('K4 §9: a hub ping changes nothing', async () => {
    const { link, socket } = connected();
    const listing = link.list('/');
    socket.text({ type: 'ping' });
    socket.text(ENTRIES);
    expect(await listing).toMatchObject({ ok: true });
  });

  it.each([
    ['a malformed message', '{"type":"entries","path":"/"}'],
    ['an unknown message', { type: 'end', reason: 'OPERATOR' }],
    ['a message not asked for', { type: 'download', size: 3 }],
    ['done for a list', { type: 'done' }],
  ])('fail closed: %s closes the link and the list ends', async (_case, message) => {
    const { link, socket } = connected();
    const listing = link.list('/');
    socket.text(message);
    expect(await listing).toEqual({ ok: false, error: { kind: 'ended' } });
    expect(link.state()).toMatchObject({ phase: 'disconnected', end: 'misread' });
    expect(socket.closedByClient).toBe(1000);
  });

  it('fail closed: entries, done or bytes with nothing asked close the link', () => {
    for (const send of [(s: FakeFilesSocket) => s.text(ENTRIES), (s: FakeFilesSocket) => s.text({ type: 'done' }), (s: FakeFilesSocket) => s.bytes(1)]) {
      const { link, socket } = connected();
      send(socket);
      expect(link.state().end).toBe('misread');
    }
  });
});

describe('one request at a time (K4 §9, D-5)', () => {
  it('D-5: while a list is in flight, another list, a download or an upload is refused locally and not sent', async () => {
    const { link, socket } = connected();
    void link.list('/');
    expect(await link.list('/etc')).toEqual({ ok: false, error: { kind: 'busy' } });
    expect(await link.download('/f')).toEqual({ ok: false, error: { kind: 'busy' } });
    expect(await link.upload('/u', new Blob(['x']))).toEqual({ ok: false, error: { kind: 'busy' } });
    expect(socket.types()).toEqual(['list']);
    socket.text(ENTRIES);
    void link.list('/etc');
    expect(socket.messages()).toEqual([
      { type: 'list', path: '/' },
      { type: 'list', path: '/etc' },
    ]);
  });

  it('K4 §9: a path the hub would refuse is not sent', async () => {
    const { link, socket } = connected();
    for (const path of ['', '/a\0b', `/${'a'.repeat(4096)}`]) expect(await link.list(path)).toEqual({ ok: false, error: { kind: 'bad-request' } });
    expect(socket.frames).toEqual([]);
  });

  it('nothing is sent before the socket opens or after the link ended', async () => {
    const { link, socket } = open();
    expect(await link.list('/')).toEqual({ ok: false, error: { kind: 'offline' } });
    socket.open();
    link.close();
    expect(await link.download('/f')).toEqual({ ok: false, error: { kind: 'offline' } });
    expect(socket.frames).toEqual([]);
  });
});
