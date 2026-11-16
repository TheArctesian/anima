// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import type { HubResult } from '$lib/hub/hub';
import type { LinkApi, LinkDevice } from './api';
import LinkDevicesSection from './LinkDevicesSection.svelte';

vi.mock('$lib/hub/context', () => ({ useHub: () => ({ request: vi.fn() }) }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const SECTION = { id: 'settings-link-devices', title: 'Devices on the link', icon: 'carbon:remote-connection' as const, description: 'The devices.' };
const LAPTOP: LinkDevice = { key: 'key-laptop', fingerprint: 'ab12-cd34', name: 'laptop', address: '100.64.0.2', path: 'RELAYED', relay: 'fra', expiresAt: null, streams: 1 };
const PHONE: LinkDevice = { ...LAPTOP, key: 'key-phone', fingerprint: 'ef56-7890', name: 'phone', path: 'DIRECT', relay: null };
const INVITE = 'anima-invite.1.aW52aXRl';
const NOW = new Date('2026-11-15T10:00:00Z');

let app: ReturnType<typeof mount> | undefined;

function fakeApi(devices: HubResult<readonly LinkDevice[]>[]) {
  const api = {
    devices: vi.fn<LinkApi['devices']>(),
    invite: vi.fn<LinkApi['invite']>(() => Promise.resolve({ ok: true, value: { invite: INVITE, expiresAt: '2026-11-15T10:15:00Z' } })),
    revoke: vi.fn<LinkApi['revoke']>(() => Promise.resolve({ ok: true, value: 1 })),
  };
  for (const answer of devices) api.devices.mockResolvedValueOnce(answer);
  api.devices.mockResolvedValue(devices.at(-1) ?? { ok: true, value: [] });
  return api;
}

async function settle(): Promise<void> {
  for (let round = 0; round < 3; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

async function open(api: ReturnType<typeof fakeApi>): Promise<HTMLElement> {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(LinkDevicesSection, { target, props: { section: SECTION, api, clock: { current: NOW } } });
  await settle();
  return target;
}

function click(label: string): void {
  const found = [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === label || button.getAttribute('aria-label') === label);
  if (found === undefined) throw new Error(`no button ${label}`);
  found.click();
  flushSync();
}

beforeEach(() => {
  installAnimations();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('FRN-009, LNK-006: the section lists each device with its key, path and expiry', async () => {
  const target = await open(fakeApi([{ ok: true, value: [LAPTOP, PHONE] }]));
  expect(target.querySelector('h2')?.textContent).toBe('Devices on the link');
  const rows = [...target.querySelectorAll('li')].map((row) => [row.getAttribute('aria-label'), row.querySelector('.mono')?.textContent, row.querySelectorAll('dd')[1]?.textContent]);
  expect(rows).toEqual([
    ['laptop', 'ab12-cd34', 'Relayed through fra'],
    ['phone', 'ef56-7890', 'Direct'],
  ]);
});

it('FRN-009: a hub with no link (UNAVAILABLE) shows no section at all', async () => {
  const target = await open(fakeApi([{ ok: false, error: { code: 'UNAVAILABLE', message: 'The link is off.' } }]));
  expect(target.querySelector('section')).toBeNull();
});

it('FRN-009, LNK-003: Invite a device asks for a name, then shows the QR code, the text and a 15-minute countdown', async () => {
  const api = fakeApi([{ ok: true, value: [] }]);
  await open(api);
  click('Invite a device');
  const name = document.querySelector<HTMLInputElement>('[role="dialog"] input');
  if (name === null) throw new Error('no name field');
  name.value = 'laptop';
  name.dispatchEvent(new Event('input', { bubbles: true }));
  click('Make an invite');
  await settle();
  expect(api.invite).toHaveBeenCalledWith('laptop');
  const dialog = document.querySelector('[role="dialog"]');
  expect(dialog?.querySelector('svg[role="img"]')?.getAttribute('aria-label')).toBe('Invite for laptop');
  expect(dialog?.querySelector('output')?.textContent).toBe(INVITE);
  expect(dialog?.querySelector('time')?.textContent).toBe('15:00');
  expect(dialog?.querySelector('[role="status"]')?.textContent.trim()).toBe('');
});

it('FRN-009, LNK-004: Revoke asks first, then revokes by key and loads the list again', async () => {
  const api = fakeApi([{ ok: true, value: [LAPTOP, PHONE] }, { ok: true, value: [PHONE] }]);
  const target = await open(api);
  click('Revoke laptop');
  expect(document.querySelector('[role="dialog"] h2')?.textContent).toBe('Revoke laptop?');
  expect(api.revoke).not.toHaveBeenCalled();
  [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((button) => button.textContent.trim() === 'Revoke laptop')?.click();
  await settle();
  expect(api.revoke).toHaveBeenCalledWith('key-laptop');
  expect([...target.querySelectorAll('li')].map((row) => row.getAttribute('aria-label'))).toEqual(['phone']);
});
