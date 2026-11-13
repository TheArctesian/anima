// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import BuildsView from '../BuildsView.svelte';
import { REFRESH_MS } from '../board.svelte';
import { fakeHub, job, ok, release, settled, type Answers } from './builds-view.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

const QUEUED = release('rel-1', '0.1.13', '2026-11-11T10:00:00Z', [job({ id: 'job-c', releaseId: 'rel-1' })]);

beforeEach(() => {
  installAnimations();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  vi.useRealTimers();
});

async function open(answers: Answers = {}): Promise<HTMLElement> {
  hub = fakeHub({
    Releases: ok({ releases: { edges: [{ node: QUEUED }] } }),
    NextReleaseVersion: ok({ nextReleaseVersion: '0.1.14' }),
    Builders: ok({ builders: [] }),
    BuildJobLog: ok({ buildJobLog: [] }),
    ...answers,
  });
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(BuildsView, { target });
  flushSync();
  await settled();
  return target;
}

/** Closes the view, as leaving the page does. */
async function close(): Promise<void> {
  if (app !== undefined) await unmount(app);
  app = undefined;
}

const count = (operation: string): number => hub.sent.filter((request) => request.operation === operation).length;

it('BLD-081: when the subscription fails the view says so and reads the releases and the open log again', async () => {
  const target = await open({ BuildJobLog: ok({ buildJobLog: [{ sequence: 1, text: 'first' }] }) });
  target.querySelector<HTMLButtonElement>('button[aria-pressed]')?.click();
  await settled();
  const [releases, logs] = [count('Releases'), count('BuildJobLog')];
  hub.fail();
  await settled();
  expect(target.textContent).toContain('Live updates stopped');
  expect(count('Releases')).toBe(releases + 1);
  expect(count('BuildJobLog')).toBe(logs + 1);
});

it('BLD-081: a subscription the hub ends is shown the same way, and Resume starts it again', async () => {
  const target = await open();
  hub.finish();
  await settled();
  expect(target.textContent).toContain('Live updates stopped');
  const resume = [...target.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Resume live updates');
  resume?.click();
  await settled();
  expect(hub.live.subscribed).toBe(2);
  expect(target.textContent).not.toContain('Live updates stopped');
});

it('BLD-081: closing the view stops its subscription', async () => {
  await open();
  expect(hub.live.stopped).toBe(0);
  await close();
  expect(hub.live.stopped).toBe(1);
});

it('BLD-007: the builders are read again each REFRESH_MS while the view is open, and no more after it closes', async () => {
  // Only the interval is faked: `settled` waits on a real timeout.
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  await open();
  const first = count('Builders');
  vi.advanceTimersByTime(REFRESH_MS);
  await settled();
  expect(count('Builders')).toBe(first + 1);
  await close();
  vi.advanceTimersByTime(REFRESH_MS * 3);
  await settled();
  expect(count('Builders')).toBe(first + 1);
});

it('BLD-007: nothing is said about waiting until the builders have loaded; a failed read says so', async () => {
  const unreachable = { ok: false, error: { code: 'unreachable', message: 'x' } } as const;
  const target = await open({ Builders: unreachable });
  expect(target.textContent).not.toContain('Waiting for a');
  expect(target.textContent).toContain('could not read the builders');
});
