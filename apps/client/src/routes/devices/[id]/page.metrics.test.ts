// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { CPU_ONLY } from '$lib/metrics/metrics.fixture';
import Page from './+page.svelte';
import { parseDeviceId } from './device-id';
import { ID, answers, device, route } from './page.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown, goto: [] as [string, unknown][] }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({ useSessions: () => ({ openSession: () => undefined }) }));
vi.mock('$app/navigation', () => ({
  goto: (href: string, options: unknown) => {
    current.goto.push([href, options]);
    return Promise.resolve();
  },
}));
vi.mock('$app/state', async () => {
  const { route: shown } = await import('./page.fixture.svelte');
  return {
    page: {
      get url() {
        return new URL(`/devices/${shown.param}${shown.search}`, 'http://localhost');
      },
      get params() {
        return { id: shown.param };
      },
      get state() {
        return shown.state;
      },
    },
  };
});

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

async function settle(): Promise<void> {
  for (let turn = 0; turn < 8; turn += 1) await tick();
  flushSync();
}

const ssh = (extra: Record<string, unknown> = {}) =>
  answers({
    Device: { device: device({ metricsSource: 'SSH' }) },
    DeviceMetrics: { device: { id: ID, metricsSource: 'SSH', gpus: [], metrics: CPU_ONLY, readings: null } },
    ...extra,
  });

async function show(hubAnswers: Record<string, unknown>, search = ''): Promise<ReturnType<typeof fakeHub>> {
  route.search = search;
  const fake = fakeHub(hubAnswers);
  current.hub = fake.hub;
  app = mount(Page, { target: document.body, props: { data: { id: parseDeviceId(route.param) } } });
  await settle();
  return fake;
}

const metricRequests = (fake: ReturnType<typeof fakeHub>) => fake.requests.filter((request) => request.operation === 'DeviceMetrics');

beforeEach(() => {
  current.goto = [];
  route.param = ID;
  route.state = {};
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('D-7: a device without SSH metrics sends no metrics request and links to Settings', async () => {
  const fake = await show(answers());
  expect(metricRequests(fake)).toEqual([]);
  expect(fake.subscriptions.map((one) => one.operation)).not.toContain('MetricsAdded');
  expect(document.body.textContent).toContain('Metrics are off for this device.');
  [...document.querySelectorAll('button')].find((one) => one.textContent.replace(/\p{Co}/gu, '').trim() === 'Open Settings')?.click();
  expect(current.goto[0]?.[0]).toBe(`/devices/${ID}?tab=settings&section=general`);
});

it('D-5: an SSH device asks for the hour first and listens for live points', async () => {
  const fake = await show(ssh());
  expect(metricRequests(fake)).toEqual([{ operation: 'DeviceMetrics', variables: { id: ID, range: 'HOUR' } }]);
  expect(fake.listeners('MetricsAdded')).toBe(1);
  expect(document.querySelectorAll('h3').length).toBeGreaterThan(0);
});

it('D-5: ?range=day asks for the day and does not listen; Back to the hour asks again', async () => {
  const fake = await show(ssh(), '?range=day');
  expect(metricRequests(fake).map((request) => request.variables)).toEqual([{ id: ID, range: 'DAY' }]);
  expect(fake.listeners('MetricsAdded')).toBe(0);
  route.search = '';
  await settle();
  expect(metricRequests(fake).map((request) => request.variables)).toEqual([
    { id: ID, range: 'DAY' },
    { id: ID, range: 'HOUR' },
  ]);
  expect(fake.listeners('MetricsAdded')).toBe(1);
});

it('D-5: choosing a range writes it to the URL; the hour leaves no query', async () => {
  await show(ssh());
  [...document.querySelectorAll('button')].find((one) => one.textContent.trim() === '7 days')?.click();
  expect(current.goto[0]?.[0]).toBe(`/devices/${ID}?range=week`);
});

it('D-6: leaving the page stops the live subscription', async () => {
  const fake = await show(ssh());
  await unmount(app ?? {});
  app = undefined;
  expect(fake.listeners('MetricsAdded')).toBe(0);
});

it('D-7: a metrics error shows the hub message and Retry asks again', async () => {
  const fake = await show(ssh({ DeviceMetrics: { code: 'INTERNAL', message: 'metrics are not served' } }));
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('metrics are not served');
  [...document.querySelectorAll('button')].find((one) => one.textContent.trim() === 'Retry')?.click();
  await settle();
  expect(metricRequests(fake)).toHaveLength(2);
});

const tabButton = (name: string) => [...document.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find((one) => one.textContent.replace(/\p{Co}/gu, '').trim() === name);

it('D-5: leaving the Overview stops the live points, and returning loads again', async () => {
  const fake = await show(ssh());
  expect(fake.listeners('MetricsAdded')).toBe(1);
  route.search = '?tab=checks';
  await settle();
  expect(fake.listeners('MetricsAdded')).toBe(0);
  route.search = '';
  await settle();
  expect(metricRequests(fake)).toHaveLength(2);
  expect(fake.listeners('MetricsAdded')).toBe(1);
});

it('D-5: the range belongs to the Overview, so choosing another tab leaves no range in the URL', async () => {
  await show(ssh(), '?range=week');
  tabButton('Checks')?.click();
  expect(current.goto[0]?.[0]).toBe(`/devices/${ID}?tab=checks`);
  route.search = '?tab=checks';
  await settle();
  tabButton('Overview')?.click();
  expect(current.goto[1]?.[0]).toBe(`/devices/${ID}`);
});

it('MET-003: a chart names its range in its text alternative, and a percent chart its 85 % line', async () => {
  await show(ssh(), '?range=day');
  const label = document.querySelector('svg[role="img"]')?.getAttribute('aria-label') ?? '';
  expect(label).toContain('CPU, last 24 hours');
  expect(label).toContain('threshold line at 85 %');
});
