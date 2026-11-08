// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { ThisMachine } from '$lib/desktop/this-machine';
import { DONT_ASK_KEY, ThisMachinePrompt } from '$lib/devices/add/this-machine';
import Page from '../../../routes/devices/+page.svelte';
import { button, input } from '../form/form/form.fixture';
import { fakeHub } from '../hub.fixture';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { device, devicesAnswer } from './screen.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const MAC: ThisMachine = { name: 'Studio', user: 'stephen', os: 'MACOS', vpnAddress: '100.64.0.7', sshListening: true };

const current = vi.hoisted(() => ({
  hub: undefined as unknown,
  desktop: true,
  machine: null as unknown,
  /** How many times the shell was asked to read this computer. */
  reads: 0,
  /** One app start: the route's prompt; a test makes a new one for a new start. */
  prompt: undefined as unknown,
}));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({ useSessions: () => ({ openSession: () => undefined }) }));
vi.mock('$app/navigation', () => ({ goto: () => Promise.resolve() }));
vi.mock('$app/state', () => ({ page: { url: new URL('/devices', 'http://localhost') } }));
vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => current.desktop,
  invoke: (command: string) => {
    if (command !== 'this_machine_read') return Promise.reject(new Error(`unexpected ${command}`));
    current.reads += 1;
    return Promise.resolve(current.machine);
  },
}));
vi.mock('$lib/devices/add/this-machine', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/devices/add/this-machine')>()),
  get thisMachinePrompt() {
    return current.prompt;
  },
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

function newStart(): void {
  current.prompt = new ThisMachinePrompt(() => localStorage);
}

async function settle(): Promise<void> {
  for (let round = 0; round < 12; round += 1) await Promise.resolve();
  flushSync();
}

/** Mounts /devices with `hosts` as the devices the hub has. */
async function visit(hosts: readonly string[] = ['nas.lan'], answers: Record<string, unknown> = {}) {
  const list = devicesAnswer(hosts.map((host, index) => device(`d${String(index)}`, { host })));
  const fake = fakeHub({ Devices: list, DeviceHosts: list, Tags: { tags: [] }, Sites: { sites: [{ id: 's1', name: 'Home' }] }, ...answers });
  current.hub = fake.hub;
  app = mount(Page, { target: document.body });
  await settle();
  return fake;
}

/** Mounts /devices with the `DeviceHosts` read held until the returned `answer` is called. */
async function visitWithHostsPending(): Promise<() => Promise<void>> {
  const list = devicesAnswer([device('d0', { host: 'nas.lan' })]);
  const fake = fakeHub({ Devices: list, Tags: { tags: [] }, Sites: { sites: [{ id: 's1', name: 'Home' }] } });
  let release: () => void = () => undefined;
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  const request = fake.hub.request;
  current.hub = {
    ...fake.hub,
    request: ((document, variables) =>
      String(JSON.stringify(document)).includes('DeviceHosts')
        ? held.then(() => ({ ok: true, value: list }))
        : request(document, variables)) as typeof request,
  };
  app = mount(Page, { target: document.body });
  await settle();
  return async () => {
    release();
    await settle();
  };
}

async function leave(): Promise<void> {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
}

const dialog = () => document.querySelector('[role="dialog"]');
const title = () => dialog()?.querySelector('h2')?.textContent;
const choice = () => document.querySelector('[role="group"][aria-label="Device to add"]');

async function openAddDevice(): Promise<void> {
  [...document.querySelectorAll('button')].find((node) => node.textContent.includes('Add device'))?.click();
  await settle();
}

beforeEach(() => {
  current.desktop = true;
  current.machine = { ...MAC };
  current.reads = 0;
  localStorage.clear();
  newStart();
});

afterEach(async () => {
  await leave();
});

it('R-060: after sign-in the desktop app asks "Add this Mac?" with this Mac chosen and filled in', async () => {
  const fake = await visit();
  expect(title()).toBe('Add this Mac?');
  expect(choice()?.querySelector('[aria-pressed="true"]')?.textContent.trim()).toBe('This Mac');
  expect(input('host').value).toBe('100.64.0.7');
  expect(fake.requests.map((request) => request.operation)).toContain('DeviceHosts');
});

it('R-060: a device whose host is the VPN address is this Mac: no prompt, and Add device is empty with no choice', async () => {
  await visit(['nas.lan', '100.64.0.7']);
  expect(dialog()).toBeNull();
  await openAddDevice();
  expect(title()).toBe('Add device');
  expect(choice()).toBeNull();
  expect(input('host').value).toBe('');
});

it('R-060: outside the desktop app there is no prompt and the hosts are not read', async () => {
  current.desktop = false;
  const fake = await visit();
  expect(dialog()).toBeNull();
  expect(fake.requests.map((request) => request.operation)).not.toContain('DeviceHosts');
});

it('R-060: a computer off the VPN is not offered', async () => {
  current.machine = { ...MAC, vpnAddress: null, sshListening: false };
  await visit();
  expect(dialog()).toBeNull();
  await openAddDevice();
  expect(choice()).toBeNull();
});

it('R-060: closing is "Not now": not asked again in this run, asked again at the next app start', async () => {
  await visit();
  button('Cancel').click();
  await settle();
  expect(dialog()).toBeNull();
  await leave();
  await visit();
  expect(dialog()).toBeNull();
  await leave();
  newStart();
  await visit();
  expect(title()).toBe('Add this Mac?');
});

it('R-060: "Don\'t ask again on this Mac" closes the modal and stops the prompt at every later start', async () => {
  await visit();
  button("Don't ask again on this Mac").click();
  await settle();
  expect(dialog()).toBeNull();
  expect(localStorage.getItem(DONT_ASK_KEY)).toBe('true');
  await leave();
  newStart();
  await visit();
  expect(dialog()).toBeNull();
});

it('R-060: Add device still offers this Mac first after the prompt was turned off', async () => {
  localStorage.setItem(DONT_ASK_KEY, 'true');
  await visit();
  expect(dialog()).toBeNull();
  await openAddDevice();
  expect(title()).toBe('Add device');
  expect(choice()?.querySelector('[aria-pressed="true"]')?.textContent.trim()).toBe('This Mac');
});

it('R-060: when the hosts cannot be read the app does not ask, since it might add a duplicate', async () => {
  await visit(['nas.lan'], { DeviceHosts: { code: 'unreachable', message: 'The hub did not answer.' } });
  expect(dialog()).toBeNull();
});

it('R-060: each opening of Add device offers this Mac first, whatever was chosen before', async () => {
  localStorage.setItem(DONT_ASK_KEY, 'true');
  await visit();
  await openAddDevice();
  button('Another device').click();
  flushSync();
  button('Cancel').click();
  await settle();
  await openAddDevice();
  expect(choice()?.querySelector('[aria-pressed="true"]')?.textContent.trim()).toBe('This Mac');
});

it('R-060: "Don\'t ask again" chosen while the hosts are still being read means no modal', async () => {
  const answer = await visitWithHostsPending();
  expect(dialog()).toBeNull();
  (current.prompt as ThisMachinePrompt).turnOff();
  await answer();
  expect(dialog()).toBeNull();
});

it('R-060: the same pending read opens the modal when the prompt is still due', async () => {
  const answer = await visitWithHostsPending();
  await answer();
  expect(title()).toBe('Add this Mac?');
});

it('R-060: once this Mac is found to be a device, later visits in the run do not read it again', async () => {
  const fake = await visit(['100.64.0.7']);
  expect(current.reads).toBe(1);
  await leave();
  const again = await visit(['100.64.0.7']);
  expect(current.reads).toBe(1);
  expect(again.requests.map((request) => request.operation)).not.toContain('DeviceHosts');
  expect(fake.requests.map((request) => request.operation)).toContain('DeviceHosts');
  await leave();
  newStart();
  await visit(['100.64.0.7']);
  expect(current.reads).toBe(2);
});
