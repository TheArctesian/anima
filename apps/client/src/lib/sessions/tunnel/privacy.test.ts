// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ending, recordedEnding } from '../ending';
import { installBlankCanvas } from './canvas.fixture';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from './socket.fixture';
import { startTunnel } from './start';
import { openTunnel, type TunnelState } from './tunnel';
import Guacamole from './vendor/guacamole.js';

const TOKEN_MARKER = 'TOKENMARKER_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const CLIPBOARD_MARKER = 'CLIPBOARDMARKER-secret-paste';
/** The keysym of a key the operator types; its decimal form must not reach a log. */
const KEYSYM_MARKER = 0x1008ff13;
const MARKERS = [TOKEN_MARKER, CLIPBOARD_MARKER, String(KEYSYM_MARKER), 'Aborted. See logs.'];

const METHODS = ['log', 'info', 'warn', 'error', 'debug', 'trace'] as const;
let restoreCanvas: () => void = () => undefined;

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  restoreCanvas = installBlankCanvas();
});

afterEach(() => {
  restoreCanvas();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function spyOnConsole() {
  return METHODS.map((method) => vi.spyOn(console, method).mockImplementation(() => undefined));
}

/** Everything written to the console, as one string per call. */
function logged(spies: ReturnType<typeof spyOnConsole>): string[] {
  return spies.flatMap((spy) => spy.mock.calls.map((args) => args.map((arg) => (typeof arg === 'string' ? arg : JSON.stringify(arg))).join(' ')));
}

describe('session paths keep secrets out of logs (LOG-007, SEC-002)', () => {
  it('LOG-007: a whole session logs no token, clipboard text, keystroke or gateway text', async () => {
    const spies = spyOnConsole();
    const states: TunnelState[] = [];
    const api = {
      startSession: () =>
        Promise.resolve({ ok: true as const, value: { sessionId: 's-1', token: TOKEN_MARKER, tunnelPath: '/tunnel', expiresAt: '2026-11-04T10:00:30Z' } }),
      endSession: () => Promise.resolve({ ok: false as const, error: { code: 'NOT_FOUND' as const, message: 'No such session.' } }),
    };
    const started = await startTunnel(api, { deviceId: 'd-1', display: { widthPx: 800, heightPx: 600 } }, {
      origin: 'https://hub.anima.ing',
      WebSocket: FAKE_TUNNEL_SOCKET,
      onchange: (state) => states.push(state),
    });
    if (!started.ok) throw new Error('not started');
    const { client } = started.value.tunnel;
    const socket = FakeTunnelSocket.latest();
    client.onclipboard = (stream) => {
      new Guacamole.StringReader(stream).ontext = () => undefined;
    };
    socket.open();
    socket.sync();
    socket.clipboard(CLIPBOARD_MARKER);
    client.sendKeyEvent(1, KEYSYM_MARKER);
    client.sendKeyEvent(0, KEYSYM_MARKER);
    const paste = new Guacamole.StringWriter(client.createClipboardStream('text/plain'));
    paste.sendText(CLIPBOARD_MARKER);
    paste.sendEnd();
    socket.gatewayError(0x0301);
    socket.closeWith(1011);
    vi.advanceTimersByTime(30_000);
    await Promise.resolve();
    const final = started.value.tunnel.state();
    const shown = recordedEnding(ending(final), 'AUTH_FAILED');

    const visible = [...logged(spies), JSON.stringify(states), JSON.stringify(final), shown.text].join('\n');
    for (const marker of MARKERS) expect(visible).not.toContain(marker);
    expect(socket.sentInstructions()).toContainEqual(['key', String(KEYSYM_MARKER), '1']);
  });

  it('LOG-007: a refused tunnel logs nothing and its error holds no token', () => {
    const spies = spyOnConsole();
    // A constructible function, as `new WebSocket` needs, that fails the way a browser does.
    const throwing = function (url: string): never {
      throw new DOMException(`Refused to connect to '${url}'`, 'SecurityError');
    } as unknown as typeof WebSocket;
    const opening = openTunnel({ origin: 'https://hub.anima.ing', ticket: { tunnelPath: '/tunnel', token: TOKEN_MARKER }, WebSocket: throwing });
    expect(JSON.stringify(opening)).not.toContain(TOKEN_MARKER);
    expect(logged(spies)).toEqual([]);
  });
});
