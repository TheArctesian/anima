import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { ListenerResult } from '$lib/desktop/hub-listener/shell';
import { createHub, parseHubAddress, type HubAddress } from '$lib/hub/hub';
import { acknowledge, FAKE_WEBSOCKET, FakeSocket, FIRST_RETRY_MS, NOW, settle } from '$lib/hub/socket/fake';
import { listenerGate } from './listener';

// Security re-review MF-1: a dead hub must not hand its token to the program that takes its port. The socket
// reconnects by itself 1 s after a drop, inside the 5 s an "own" answer stands, so its reader asks the shell afresh.

const parsed = parseHubAddress('http://127.0.0.1:8443');
if (!parsed.ok) throw new Error('no address');
const ADDRESS: HubAddress = parsed.value;
const TOKEN = 'tok-secret';

let clock = 0;
const check = vi.fn<(origin: string) => Promise<ListenerResult>>();

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  FakeSocket.opened = [];
  clock = 0;
  check.mockReset().mockResolvedValue({ ok: true, listener: 'own' });
});

afterEach(() => {
  vi.useRealTimers();
});

/** The hub as the root layout makes it: one gate, a kept answer for requests and a fresh one for each connection. */
function layoutHub() {
  const gate = listenerGate({ check, now: () => clock });
  const reader = vi.fn(() => Promise.resolve(TOKEN));
  const hub = createHub({
    address: ADDRESS,
    token: gate.token(reader),
    socketToken: gate.token(reader, { fresh: true }),
    WebSocket: FAKE_WEBSOCKET,
    now: () => NOW,
  });
  return { hub, reader };
}

it('MF-1, F-10: after a drop, the reconnect to a port another program has taken carries no token', async () => {
  const { hub } = layoutHub();
  await settle();
  const first = FakeSocket.opened[0];
  if (first === undefined) throw new Error('no socket');
  await acknowledge(first);
  expect(JSON.stringify(first.sent)).toContain(`Bearer ${TOKEN}`);

  clock += 3000;
  await settle(3000);
  first.close(1006);
  check.mockResolvedValue({ ok: true, listener: 'other' });
  clock += FIRST_RETRY_MS;
  await settle(FIRST_RETRY_MS);
  const second = FakeSocket.opened[1];
  if (second === undefined) throw new Error('no reconnect');
  second.accept();
  await settle();

  expect(check).toHaveBeenCalledTimes(2);
  expect(JSON.stringify(second.sent)).not.toContain(TOKEN);
  await hub.close();
});

it('MF-1: the socket asks the shell for each connection; requests may use the kept answer', async () => {
  const { hub } = layoutHub();
  await settle();
  const first = FakeSocket.opened[0];
  if (first === undefined) throw new Error('no socket');
  await acknowledge(first);
  first.close(1006);
  await settle(FIRST_RETRY_MS);
  const second = FakeSocket.opened[1];
  if (second === undefined) throw new Error('no reconnect');
  await acknowledge(second);
  expect(check).toHaveBeenCalledTimes(2);
  expect(JSON.stringify(second.sent)).toContain(`Bearer ${TOKEN}`);
  await hub.close();
});
