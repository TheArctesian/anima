// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { readings } from '../../metrics/bars/bars.fixture';
import { fakeHub } from '../hub.fixture';
import { DEFAULT_QUERY } from '../query';
import DeviceScreen from '../screen/DeviceScreen.svelte';
import { CARDS, device, devicesAnswer } from '../screen/screen.fixture.svelte';
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

const LAPTOP = device('laptop', { metricsSource: 'DAEMON', readings: readings(), batteries: [{ name: 'BAT0', state: 'DISCHARGING' }] });
const SERVER = device('server', { metricsSource: 'DAEMON', readings: readings() });

async function show(query = CARDS) {
  const fake = fakeHub({ Devices: devicesAnswer([LAPTOP, SERVER]), Tags: { tags: [] } });
  current.hub = fake.hub;
  app = mount(DeviceScreen, {
    target: document.body,
    props: { query, onQueryChange: () => undefined, onAddDevice: () => undefined, onConnect: () => undefined },
  });
  await settle();
  return fake;
}

function charge(fake: ReturnType<typeof fakeHub>, value: number) {
  fake.push('BatteryChargesAdded', {
    metricsAdded: { deviceId: 'laptop', samples: [{ kind: 'CPU_PERCENT', label: '', value: 3 }, { kind: 'BATTERY_PERCENT', label: 'BAT0', value }] },
  });
  flushSync();
}

const batteryOf = (scope: Element | null | undefined) => scope?.querySelector('.meter.battery');

it('MET-032: a tile shows the charge of its battery beside its meters, and a tile with no battery shows nothing new', async () => {
  const fake = await show();
  const [laptop, server] = [...document.querySelectorAll('article')];
  expect(batteryOf(laptop)?.textContent).toContain('—');
  charge(fake, 64);
  expect(batteryOf(laptop)?.getAttribute('aria-valuenow')).toBe('64');
  expect(batteryOf(laptop)?.getAttribute('aria-valuetext')).toBe('64 %, on battery');
  expect(batteryOf(server)).toBeNull();
  expect(server?.querySelectorAll('[role="meter"]')).toHaveLength(3);
});

it('MET-032: under 20 % while it discharges the meter warns in words and in its shape', async () => {
  const fake = await show();
  charge(fake, 18);
  const meter = batteryOf(document.querySelector('article'));
  expect(meter?.classList.contains('degraded')).toBe(true);
  expect(meter?.getAttribute('aria-valuetext')).toBe('18 %, low, on battery');
});

it('MET-032: the list has a Battery head only when a listed device has a battery, and the battery is a fourth slot', async () => {
  const fake = await show(DEFAULT_QUERY);
  charge(fake, 80);
  const head = document.querySelector('thead .c-detail');
  expect(head?.textContent).toContain('Battery');
  expect(head?.getAttribute('aria-label')).toBe('Usage: CPU, memory, disk, battery');
  const rows = [...document.querySelectorAll('tbody tr.device')];
  expect(rows.map((row) => row.querySelectorAll('[role="meter"]').length)).toEqual([4, 3]);
});

it('MET-032: a list with no battery keeps its three usage heads', async () => {
  const fake = fakeHub({ Devices: devicesAnswer([SERVER]), Tags: { tags: [] } });
  current.hub = fake.hub;
  app = mount(DeviceScreen, {
    target: document.body,
    props: { query: DEFAULT_QUERY, onQueryChange: () => undefined, onAddDevice: () => undefined, onConnect: () => undefined },
  });
  await settle();
  expect(document.querySelector('thead .c-detail')?.getAttribute('aria-label')).toBe('Usage: CPU, memory, disk');
});
