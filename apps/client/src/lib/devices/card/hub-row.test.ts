// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { readings } from '../../metrics/bars/bars.fixture';
import { fakeHub } from '../hub.fixture';
import { DEFAULT_QUERY } from '../query';
import DeviceScreen from '../screen/DeviceScreen.svelte';
import { CARDS, device, devicesAnswer } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

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

const GIB = 1024 ** 3;
// The container of a hosted hub: Linux, no way in, CPU and memory of its own control group (HMD-071).
const HUB = device('hub', { kind: 'hub', name: 'Hub', os: 'LINUX', metricsSource: 'DAEMON', readings: readings({ cpuPercent: 12, memoryPercent: 40, diskPercent: null, diskMount: null }) });
const NAS = device('nas', { os: 'LINUX', metricsSource: 'DAEMON', readings: readings() });

async function show(query = CARDS) {
  const { fakeCalls } = await import('../screen-sharing/calls.fixture');
  const fake = fakeHub({ Devices: devicesAnswer([HUB, NAS]), Tags: { tags: [] } });
  current.hub = fake.hub;
  app = mount(DeviceScreen, {
    target: document.body,
    props: { query, onQueryChange: () => undefined, onAddDevice: () => undefined, onConnect: () => undefined, screenSharing: fakeCalls().calls },
  });
  await settle();
  return fake;
}

function batch(fake: ReturnType<typeof fakeHub>, bytes: number) {
  fake.push('ContainerSamplesAdded', { metricsAdded: { deviceId: 'hub', samples: [{ kind: 'MEMORY_BYTES', label: '', value: bytes }] } });
  flushSync();
}

const meterOf = (scope: Element | null | undefined, key: string) => scope?.querySelector(`.meter.${key}`);
const labels = (scope: Element | null | undefined) => [...(scope?.querySelectorAll('button') ?? [])].map((node) => node.getAttribute('aria-label') ?? node.textContent.trim());

it('HMD-073: the tile of the hub shows CPU and its memory as used of its limit, and no disk', async () => {
  const fake = await show();
  const [hub, nas] = [...document.querySelectorAll('article')];
  expect(fake.subscriptions).toContainEqual({ operation: 'ContainerSamplesAdded', variables: { deviceId: 'hub' } });
  expect(meterOf(hub, 'memory')?.textContent).toContain('40 %');
  batch(fake, 0.8 * GIB);
  expect(meterOf(hub, 'memory')?.textContent).toContain('0.8 of 2.0 GiB');
  expect(meterOf(hub, 'memory')?.getAttribute('aria-valuetext')).toBe('0.8 of 2.0 GiB, 40 %');
  expect(meterOf(hub, 'cpu')?.getAttribute('title')).toBe('CPU: the share of the processors that the hub may use');
  expect(meterOf(hub, 'disk')).toBeNull();
  expect(nas?.querySelectorAll('[role="meter"]')).toHaveLength(3);
});

it('HMD-046: the row of the hub offers no protocol and no set-up of screen sharing; a Linux row beside it does', async () => {
  const fake = await show(DEFAULT_QUERY);
  batch(fake, 0.8 * GIB);
  const [hub, nas] = [...document.querySelectorAll('tbody tr.device')];
  expect(labels(hub?.querySelector('.c-connect'))).toEqual([]);
  expect(labels(nas?.querySelector('.c-connect'))).toContain('Set up screen sharing for Device nas');
  expect(meterOf(hub, 'memory')?.textContent).toContain('0.8 of 2.0 GiB');
  expect(meterOf(hub, 'disk')).toBeNull();
});
