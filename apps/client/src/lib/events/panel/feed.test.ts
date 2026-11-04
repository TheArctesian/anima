// @vitest-environment jsdom
import { beforeEach, expect, it } from 'vitest';
import { DEFAULT_LOG_FILTER } from '../log';
import { LogFeed } from './feed.svelte';
import { NOW, deferredHub, event, page } from './panel.fixture';

let hub: ReturnType<typeof deferredHub>;
let feed: LogFeed;
const ids = () => feed.log?.rows.map((row) => row.id);

beforeEach(() => {
  hub = deferredHub();
  feed = new LogFeed(hub.hub, () => NOW);
});

const push = (over: ReturnType<typeof event>) => {
  hub.push('EventAdded', { eventAdded: over });
};

async function later(): Promise<void> {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
}

it('LOG-009: an answer to an earlier filter is dropped when a later one has arrived', async () => {
  feed.start();
  feed.setFilter({ ...DEFAULT_LOG_FILTER, since: '1h' });
  hub.pending[1]?.answer(page([event('new', 1)]));
  await later();
  hub.pending[0]?.answer(page([event('old', 1)]));
  await later();
  expect(ids()).toEqual(['new']);
});

it('LOG-009: an older page that arrives after a filter change is dropped', async () => {
  feed.start();
  hub.pending[0]?.answer(page([event('a', 1)], 'c1'));
  await later();
  const older = feed.loadOlder();
  feed.setFilter({ ...DEFAULT_LOG_FILTER, since: '1h' });
  hub.pending[2]?.answer(page([event('b', 1)]));
  await later();
  hub.pending[1]?.answer(page([event('stale', 2)]));
  await older;
  await later();
  expect(ids()).toEqual(['b']);
});

it('LOG-010: an answer that comes after stop() shows nothing', async () => {
  feed.start();
  feed.stop();
  hub.pending[0]?.answer(page([event('a', 1)]));
  await later();
  expect(feed.log).toBeNull();
  expect(hub.listeners('EventAdded')).toBe(0);
});

it('LOG-010: a live event that comes while the page loads joins the page when it arrives', async () => {
  feed.start();
  push(event('live', 0));
  hub.pending[0]?.answer(page([event('a', 1)]));
  await later();
  expect(ids()).toEqual(['live', 'a']);
});

it('LOG-010: a live event held during a filter change is judged by the new filter', async () => {
  feed.start();
  hub.pending[0]?.answer(page([event('a', 1)]));
  await later();
  feed.setFilter({ ...DEFAULT_LOG_FILTER, deviceId: 'd2' });
  push(event('match', 0, { deviceId: 'd2' }));
  push(event('miss', 0, { deviceId: 'd1' }));
  hub.pending[1]?.answer(page([event('b', 1, { deviceId: 'd2' })]));
  await later();
  expect(ids()).toEqual(['match', 'b']);
});

it('LOG-009: a filter change clears the old rows, and a failed reload leaves the error and no stale rows', async () => {
  feed.start();
  hub.pending[0]?.answer(page([event('a', 1)]));
  await later();
  feed.setFilter({ ...DEFAULT_LOG_FILTER, since: '1h' });
  expect(feed.log).toBeNull();
  hub.pending[1]?.answer({ ok: false, error: { code: 'unreachable', message: 'No hub.' } });
  await later();
  expect(feed.error?.message).toBe('No hub.');
  expect(feed.log).toBeNull();
});
