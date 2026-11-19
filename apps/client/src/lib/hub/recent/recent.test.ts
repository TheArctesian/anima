import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { parse } from 'graphql';
import { afterEach, describe, expect, it } from 'vitest';
import { createHub, NO_HUB_ADDRESS, parseHubAddress, TOKEN_UNREADABLE, UNREACHABLE, type Hub, type HubAddress } from '../hub';
import { FAKE_WEBSOCKET } from '../socket/fake';
import type { CopyLimits, CopyStore } from './copy';
import { memoryCopyStore } from './memory.fixture';

type Named = TypedDocumentNode<Record<string, unknown>, { id?: string }>;
const DEVICE = parse('query Device($id: ID!) { device(id: $id) { name } }') as Named;
const API_KEYS = parse('query ApiKeys { apiKeys { name } }') as Named;
const RENAME = parse('mutation Device($id: ID!) { renameDevice(id: $id) { name } }') as Named;

const LIMITS: CopyLimits = { most: 2, longest: 200, oldestMs: 60 * 60_000 };
const START = new Date('2026-11-18T10:00:00Z');
const minutesLater = (minutes: number) => new Date(START.getTime() + minutes * 60_000);

/** The network of one test: `answer` is what the hub says next, or `null` when it does not answer. */
function network() {
  const state: { answer: unknown; now: Date } = { answer: null, now: START };
  const fakeFetch: typeof fetch = () =>
    state.answer === null ? Promise.reject(new TypeError('no network')) : Promise.resolve(new Response(JSON.stringify(state.answer)));
  return { state, fakeFetch };
}

const hubs: Hub[] = [];
afterEach(async () => {
  await Promise.all(hubs.splice(0).map((hub) => hub.close()));
});

/** The era of the copy in a test; a test that removes the copy adds one. */
const era = { current: 0 };

function hubWith(store: CopyStore | undefined, net = network(), address: HubAddress = NO_HUB_ADDRESS, token = () => Promise.resolve('token')) {
  const hub = createHub({
    address,
    token,
    fetch: net.fakeFetch,
    WebSocket: FAKE_WEBSOCKET,
    now: () => net.state.now,
    ...(store === undefined ? {} : { recent: { store, limits: LIMITS, era: () => era.current } }),
  });
  hubs.push(hub);
  return { hub, state: net.state };
}

const named = (name: string) => ({ data: { device: { name } } });

describe('the copy of recent hub answers (FND-007)', () => {
  it('gives the last answer of a copied query when the hub does not answer, and the status holds its time', async () => {
    const { hub, state } = hubWith(memoryCopyStore());
    state.answer = named('mini');
    await hub.request(DEVICE, { id: 'd1' });
    state.answer = null;
    state.now = minutesLater(5);
    expect(await hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: true, value: { device: { name: 'mini' } } });
    expect(hub.status.current).toEqual({ connection: 'unreachable', lastContactAt: START, copyFrom: START });
  });

  it('a start with no hub shows the copy that an earlier run of the app kept', async () => {
    const store = memoryCopyStore();
    const earlier = hubWith(store);
    earlier.state.answer = named('mini');
    await earlier.hub.request(DEVICE, { id: 'd1' });
    const { hub, state } = hubWith(store);
    state.now = minutesLater(30);
    expect(await hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: true, value: { device: { name: 'mini' } } });
    expect(hub.status.current).toEqual({ connection: 'unreachable', lastContactAt: null, copyFrom: START });
  });

  it('the status keeps the time of the oldest copy shown, and drops it when the hub answers again', async () => {
    const { hub, state } = hubWith(memoryCopyStore());
    state.answer = named('one');
    await hub.request(DEVICE, { id: 'd1' });
    state.now = minutesLater(10);
    state.answer = named('two');
    await hub.request(DEVICE, { id: 'd2' });
    state.answer = null;
    await hub.request(DEVICE, { id: 'd2' });
    await hub.request(DEVICE, { id: 'd1' });
    await hub.request(DEVICE, { id: 'd2' });
    expect(hub.status.current.copyFrom).toEqual(START);
    state.answer = named('three');
    state.now = minutesLater(20);
    await hub.request(DEVICE, { id: 'd1' });
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: minutesLater(20), copyEnds: 1 });
  });

});

describe('the age of the copy in the status (FND-007)', () => {
  it('counts each time the hub answers again after a copy was shown, so that the pages load again', async () => {
    const { hub, state } = hubWith(memoryCopyStore());
    state.answer = named('mini');
    await hub.request(DEVICE, { id: 'd1' });
    await hub.request(DEVICE, { id: 'd1' });
    expect(hub.status.current.copyEnds).toBeUndefined();
    for (const ends of [1, 2]) {
      state.answer = null;
      await hub.request(DEVICE, { id: 'd1' });
      state.answer = named('mini');
      await hub.request(API_KEYS, {});
      expect(hub.status.current.copyEnds).toBe(ends);
    }
    state.answer = null;
    await hub.request(API_KEYS, {});
    state.answer = named('mini');
    await hub.request(API_KEYS, {});
    expect(hub.status.current.copyEnds).toBe(2);
  });

  it('a copy answers only for the same variables and the same hub', async () => {
    const store = memoryCopyStore();
    const first = hubWith(store);
    first.state.answer = named('mini');
    await first.hub.request(DEVICE, { id: 'd1' });
    first.state.answer = null;
    expect(await first.hub.request(DEVICE, { id: 'd2' })).toEqual({ ok: false, error: UNREACHABLE });
    const elsewhere = parseHubAddress('https://other.example');
    if (!elsewhere.ok) throw new Error('the test address must parse');
    const other = hubWith(store, network(), elsewhere.value);
    expect(await other.hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: false, error: UNREACHABLE });
  });

});

describe('what the copy holds (FND-007)', () => {
  it('keeps no answer of a query that is not on the list, and none of a mutation with a listed name', async () => {
    const store = memoryCopyStore();
    const { hub, state } = hubWith(store);
    state.answer = { data: { apiKeys: [{ name: 'k' }] } };
    await hub.request(API_KEYS, {});
    state.answer = { data: { renameDevice: { name: 'n' } } };
    await hub.request(RENAME, { id: 'd1' });
    expect(store.copies.size).toBe(0);
    state.answer = null;
    expect(await hub.request(RENAME, { id: 'd1' })).toEqual({ ok: false, error: UNREACHABLE });
  });

  it('the event log finds its copy though the request asks from a moment that moves with the clock', async () => {
    const events = parse('query Events($filter: EventFilter) { events(filter: $filter) { totalCount } }') as TypedDocumentNode<
      Record<string, unknown>,
      { filter: { since: string; deviceId: string } }
    >;
    const { hub, state } = hubWith(memoryCopyStore());
    state.answer = { data: { events: { totalCount: 7 } } };
    await hub.request(events, { filter: { since: '2026-11-17T10:00:00.000Z', deviceId: 'd1' } });
    state.answer = null;
    const later = await hub.request(events, { filter: { since: '2026-11-17T10:05:00.123Z', deviceId: 'd1' } });
    expect(later).toEqual({ ok: true, value: { events: { totalCount: 7 } } });
    expect((await hub.request(events, { filter: { since: '2026-11-17T10:05:00.123Z', deviceId: 'd2' } })).ok).toBe(false);
  });

  it('keeps no checks of a device: the settings of a check can hold a credential', async () => {
    const checks = parse('query DeviceChecks($id: ID!) { device(id: $id) { checks { config { pushPath } } } }') as Named;
    const store = memoryCopyStore();
    const { hub, state } = hubWith(store);
    state.answer = { data: { device: { checks: [{ config: { pushPath: '/push/secret-token' } }] } } };
    await hub.request(checks, { id: 'd1' });
    expect(store.copies.size).toBe(0);
  });

});

describe('when the copy is removed or not kept (FND-007)', () => {
  it('removes the copy when the hub refuses the token, and keeps it when this computer could not read the token', async () => {
    const store = memoryCopyStore();
    const { hub, state } = hubWith(store);
    state.answer = named('mini');
    await hub.request(DEVICE, { id: 'd1' });
    const unreadable = hubWith(store, network(), NO_HUB_ADDRESS, () => Promise.reject(new Error('key store')));
    expect(await unreadable.hub.request(API_KEYS, {})).toEqual({ ok: false, error: TOKEN_UNREADABLE });
    expect(store.copies.size).toBe(1);
    state.answer = { errors: [{ message: 'Sign in again.', extensions: { code: 'UNAUTHENTICATED' } }] };
    await hub.request(API_KEYS, {});
    expect(store.copies.size).toBe(0);
  });

  it('does not keep an answer to a request that was sent before the copy was removed', async () => {
    const store = memoryCopyStore();
    const net = network();
    let release: (response: Response) => void = () => undefined;
    const slowFetch: typeof fetch = () => new Promise((resolve) => (release = resolve));
    const { hub } = hubWith(store, { ...net, fakeFetch: slowFetch });
    const pending = hub.request(DEVICE, { id: 'd1' });
    await Promise.resolve();
    await Promise.resolve();
    era.current += 1;
    release(new Response(JSON.stringify(named('mini'))));
    expect((await pending).ok).toBe(true);
    expect(store.copies.size).toBe(0);
  });

  it('an error from the hub is given as it is, with no copy in its place', async () => {
    const { hub, state } = hubWith(memoryCopyStore());
    state.answer = named('mini');
    await hub.request(DEVICE, { id: 'd1' });
    state.answer = { errors: [{ message: 'Sign in again.', extensions: { code: 'UNAUTHENTICATED' } }] };
    expect(await hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: false, error: { code: 'UNAUTHENTICATED', message: 'Sign in again.' } });
  });

});

describe('the limits of the copy (FND-007)', () => {
  it('does not show a copy that is older than the limit', async () => {
    const { hub, state } = hubWith(memoryCopyStore());
    state.answer = named('mini');
    await hub.request(DEVICE, { id: 'd1' });
    state.answer = null;
    state.now = minutesLater(61);
    expect(await hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: false, error: UNREACHABLE });
  });

  it('keeps the newest answers up to the limit, and drops an answer that became too long', async () => {
    const store = memoryCopyStore();
    const { hub, state } = hubWith(store);
    for (const [minute, id] of [[0, 'd1'], [1, 'd2'], [2, 'd3']] as const) {
      state.now = minutesLater(minute);
      state.answer = named(id);
      await hub.request(DEVICE, { id });
    }
    state.answer = named('x'.repeat(300));
    await hub.request(DEVICE, { id: 'd3' });
    state.answer = null;
    const shown = await Promise.all(['d1', 'd2', 'd3'].map(async (id) => (await hub.request(DEVICE, { id })).ok));
    expect(shown).toEqual([false, true, false]);
  });

  it('a store that fails changes no answer: a good answer stays good and no answer stays unreachable', async () => {
    const { hub, state } = hubWith(memoryCopyStore(true));
    state.answer = named('mini');
    expect(await hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: true, value: { device: { name: 'mini' } } });
    state.answer = null;
    expect(await hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: false, error: UNREACHABLE });
  });

  it('a hub with no copy store gives unreachable as before', async () => {
    const { hub, state } = hubWith(undefined);
    state.answer = named('mini');
    await hub.request(DEVICE, { id: 'd1' });
    state.answer = null;
    expect(await hub.request(DEVICE, { id: 'd1' })).toEqual({ ok: false, error: UNREACHABLE });
  });
});
