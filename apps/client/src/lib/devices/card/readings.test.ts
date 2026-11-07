// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { readings } from '../../metrics/bars/bars.fixture';
import { fakeHub } from '../hub.fixture';
import { DEFAULT_QUERY } from '../query';
import DeviceScreen from '../screen/DeviceScreen.svelte';
import { device, devicesAnswer } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => ({ reduced: false }) }));
const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

async function settle(): Promise<void> {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
}

async function show(devices: Parameters<typeof devicesAnswer>[0]) {
  const fake = fakeHub({ Devices: devicesAnswer(devices), Tags: { tags: [] } });
  current.hub = fake.hub;
  app = mount(DeviceScreen, {
    target: document.body,
    props: { query: DEFAULT_QUERY, onQueryChange: () => undefined, onAddDevice: () => undefined, onConnect: () => undefined },
  });
  await settle();
  return fake;
}

const meters = (card: Element | null | undefined) =>
  [...(card?.querySelectorAll('[role="meter"]') ?? [])].map((node) => node.getAttribute('aria-valuenow'));

it('MET-004: a card draws the readings the Devices query returned, and says so when a device has none', async () => {
  await show([device('a', { metricsSource: 'SSH', readings: readings() }), device('b', { metricsSource: 'NONE', readings: null })]);
  const cards = [...document.querySelectorAll('article')];
  expect(meters(cards[0])).toEqual(['10', '20', '30']);
  expect(cards[1]?.textContent).toContain('No metrics');
});

it('MET-004 MET-008: a pushed reading replaces the matching card only, and keeps the list as it is', async () => {
  const fake = await show([device('a', { metricsSource: 'SSH', readings: readings() }), device('b', { metricsSource: 'SSH', readings: readings() })]);
  expect(fake.subscriptions.find((call) => call.operation === 'CardReadingsAdded')?.variables).toEqual({});
  fake.push('CardReadingsAdded', {
    metricsAdded: { deviceId: 'b', readings: readings({ cpuPercent: 91, memoryPercent: 5, diskPercent: 6 }) },
  });
  await settle();
  const cards = [...document.querySelectorAll('article')];
  expect(meters(cards[0])).toEqual(['10', '20', '30']);
  expect(meters(cards[1])).toEqual(['91', '5', '6']);
  expect(cards[1]?.querySelector('.degraded')).not.toBeNull();
});

it('MET-004: a reading for a device that is not listed changes nothing', async () => {
  const fake = await show([device('a', { metricsSource: 'SSH', readings: readings() })]);
  fake.push('CardReadingsAdded', { metricsAdded: { deviceId: 'zzz', readings: readings({ cpuPercent: 99 }) } });
  await settle();
  expect(meters(document.querySelector('article'))).toEqual(['10', '20', '30']);
});
