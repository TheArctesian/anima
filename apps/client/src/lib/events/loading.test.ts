// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { threshold } from '$lib/motion/tokens';
import { LogFeed } from './panel/feed.svelte';
import LogList from './panel/LogList.svelte';
import { NOW, deferredHub, event, page, settle } from './panel/panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

/** A list whose first page waits until `answer()` is called. */
async function slowList() {
  const fake = deferredHub();
  const feed = new LogFeed(fake.hub, () => NOW);
  const time = manualClock();
  app = mount(LogList, { target: document.body, props: { feed, now: () => NOW, clock: time.clock } });
  feed.start();
  await settle();
  return {
    time,
    feed,
    answer: async () => {
      fake.pending[0]?.answer(page([event('e2', 1), event('e1', 2)]));
      await settle();
    },
  };
}

const places = () => document.querySelectorAll('.places li.row.place');
const messages = () => [...document.querySelectorAll('ol[aria-label="Log events"] .message')].map((node) => node.textContent);

it('Loading: the log list shows no skeleton before threshold-loading and placeholder rows after it', async () => {
  const { time } = await slowList();
  time.advance(threshold.loading - 1);
  flushSync();
  expect(places()).toHaveLength(0);
  time.advance(1);
  flushSync();
  expect(places().length).toBeGreaterThan(0);
  expect(document.querySelector('.places')?.getAttribute('aria-hidden')).toBe('true');
});

it('Loading: log rows that arrive before threshold-loading show at once and the skeleton never does', async () => {
  const { time, answer } = await slowList();
  time.advance(threshold.loading - 1);
  await answer();
  time.advance(threshold.loading * 2);
  flushSync();
  expect(places()).toHaveLength(0);
  expect(messages()).toEqual(['message e2', 'message e1']);
});

it('Loading: log rows that arrive under the skeleton wait for threshold-loading-min before it leaves', async () => {
  const { time, answer } = await slowList();
  time.advance(threshold.loading);
  flushSync();
  await answer();
  time.advance(threshold.loadingMin - 1);
  flushSync();
  expect(places().length).toBeGreaterThan(0);
  time.advance(1);
  await settle();
  await vi.waitFor(() => {
    expect(places()).toHaveLength(0);
  });
  expect(messages()).toEqual(['message e2', 'message e1']);
});

it('Loading: the skeleton shares the rows cell and the row class, so a row and its placeholder have one geometry', async () => {
  const { time } = await slowList();
  time.advance(threshold.loading);
  flushSync();
  const stack = document.querySelector('.stack');
  expect(stack?.querySelectorAll(':scope > ol')).toHaveLength(2);
  expect(places().length).toBe(document.querySelectorAll('.places > li.row').length);
});

it('Loading: a failed first page ends the loading state, so no skeleton stays', async () => {
  const fake = deferredHub();
  const feed = new LogFeed(fake.hub, () => NOW);
  const time = manualClock();
  app = mount(LogList, { target: document.body, props: { feed, now: () => NOW, clock: time.clock } });
  feed.start();
  await settle();
  time.advance(threshold.loading);
  flushSync();
  fake.pending[0]?.answer({ ok: false, error: { kind: 'network', message: 'down' } } as never);
  await settle();
  time.advance(threshold.loadingMin);
  await settle();
  await vi.waitFor(() => {
    expect(places()).toHaveLength(0);
  });
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
});
