// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import type { CardDevice } from '$lib/devices/api';
import type { WakeApi } from '$lib/sessions/wake/api';
import DeviceCard from '../card/DeviceCard.svelte';
import { device } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { canWake } from './actions';

endTransitionsAtOnce();
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => ({ reduced: false }) }));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const api: WakeApi = { wakeDevice: vi.fn(() => Promise.resolve({ ok: true as const, value: { sentAt: 'now', relayName: 'pi' } })) };

function show(over: Partial<CardDevice>, wake: WakeApi | undefined = api): HTMLButtonElement | undefined {
  app = mount(DeviceCard, {
    target: document.body,
    props: { device: device('a', over), beats: undefined, onConnect: vi.fn(), wake },
  });
  flushSync();
  return [...document.querySelectorAll('button')].find((node) => node.textContent.includes('Wake'));
}

it('RA-031: a device with a MAC address has a Wake button, enabled while it is not Up', () => {
  expect(show({ macAddress: 'aa:bb:cc:dd:ee:ff', status: 'DOWN' })?.disabled).toBe(false);
});

it('RA-031: the Wake button is disabled while the device is Up', () => {
  expect(show({ macAddress: 'aa:bb:cc:dd:ee:ff', status: 'UP' })?.disabled).toBe(true);
});

it('RA-031: a device without a MAC address has no Wake button', () => {
  expect(show({ macAddress: null, status: 'DOWN' })).toBeUndefined();
  expect(canWake({ macAddress: '' })).toBe(false);
});

it('RA-031: no route wake call means no Wake button', () => {
  app = mount(DeviceCard, {
    target: document.body,
    props: { device: device('a', { macAddress: 'aa:bb:cc:dd:ee:ff', status: 'DOWN' }), beats: undefined, onConnect: vi.fn() },
  });
  flushSync();
  expect(document.body.textContent).not.toContain('Wake');
});

it('RA-031: Wake asks the hub to wake this device', () => {
  show({ macAddress: 'aa:bb:cc:dd:ee:ff', status: 'DOWN' })?.click();
  expect(api.wakeDevice).toHaveBeenCalledWith('a');
});
