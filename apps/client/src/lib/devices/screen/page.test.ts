// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Page from '../../../routes/devices/+page.svelte';
import { button, type } from '../form/form/form.fixture';
import { fakeHub } from '../hub.fixture';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { device, devicesAnswer, url } from './screen.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown, goto: [] as [string, unknown][], opened: [] as unknown[] }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({
  useSessions: () => ({ openSession: (request: unknown) => current.opened.push(request) }),
}));

vi.mock('$app/navigation', () => ({
  goto: (href: string, options: unknown) => {
    current.goto.push([href, options]);
    return Promise.resolve();
  },
}));

vi.mock('$app/state', async () => {
  const { url: state } = await import('./screen.fixture.svelte');
  const { writeListQuery } = await import('../query');
  return {
    page: {
      get url() {
        return new URL(`/devices?${writeListQuery(state.query).toString()}`, 'http://localhost');
      },
    },
  };
});

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

const SITES = { sites: [{ id: 's1', name: 'Home' }] };

async function show(devices = [device('a')], answers: Record<string, unknown> = {}): Promise<ReturnType<typeof fakeHub>> {
  const fake = fakeHub({ Devices: devicesAnswer(devices), Tags: { tags: [] }, Sites: SITES, ...answers });
  current.hub = fake.hub;
  app = mount(Page, { target: document.body });
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
  return fake;
}

beforeEach(() => {
  current.goto = [];
  current.opened = [];
  url.query = { view: 'grid', name: '', statuses: [], tags: [], sort: 'NAME' };
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.useRealTimers();
});

it('DEV-001: the page has one h1 and the list in a labelled list', async () => {
  await show();
  expect(document.querySelectorAll('h1')).toHaveLength(1);
  expect(document.querySelector('h1')?.textContent).toBe('Devices');
  expect(document.querySelectorAll('article')).toHaveLength(1);
});

it('DEV-003: a filter writes the URL query with replaceState, keeping focus and scroll', async () => {
  await show();
  const toggle = document.querySelectorAll<HTMLButtonElement>('.views button')[1];
  toggle?.click();
  expect(current.goto).toEqual([['/devices?view=table', { replaceState: true, keepFocus: true, noScroll: true }]]);
});

it('DEV-003: the URL query is what the page shows (Back and Forward restore the view)', async () => {
  await show();
  url.query = { ...url.query, view: 'table' };
  flushSync();
  expect(document.querySelector('table')).not.toBeNull();
  url.query = { ...url.query, view: 'grid' };
  flushSync();
  expect(document.querySelector('table')).toBeNull();
});

const addButton = () => [...document.querySelectorAll('button')].find((node) => node.textContent.includes('Add device'));
const dialog = () => document.querySelector('[role="dialog"]');

async function openAddDevice(): Promise<void> {
  addButton()?.click();
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
}

it('WP-13 H-1, DEV-010: Add device opens the form in a modal over the list, not a new route', async () => {
  await show();
  await openAddDevice();
  expect(dialog()?.textContent).toContain('Add device');
  expect(current.goto).toEqual([]);
});

it('WP-13 H-1: a device created in the modal opens its detail page', async () => {
  const detail = { id: 'new 1', name: 'nas', host: 'nas.lan' };
  const fake = await show([device('a')], { CreateDevice: { createDevice: detail } });
  await openAddDevice();
  type('name', 'nas');
  type('host', 'nas.lan');
  [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((node) => node.textContent.trim() === 'Save')?.click();
  for (let round = 0; round < 10; round += 1) await Promise.resolve();
  flushSync();
  expect(fake.requests.find((request) => request.operation === 'CreateDevice')?.variables).toMatchObject({
    input: { siteId: 's1', name: 'nas', host: 'nas.lan', addDefaultCheck: true },
  });
  expect(current.goto.map(([href]) => href)).toEqual(['/devices/new%201']);
});

const DETAIL = { id: 'd9', name: 'nas', host: 'nas.lan' };

async function settleAll(): Promise<void> {
  for (let round = 0; round < 10; round += 1) await Promise.resolve();
  flushSync();
}

/** Opens the modal, fills a device with SSH on and a typed password, and presses Save. */
async function addWithPassword(answers: Record<string, unknown>): Promise<ReturnType<typeof fakeHub>> {
  const fake = await show([device('a')], { CreateDevice: { createDevice: DETAIL }, ...answers });
  await openAddDevice();
  type('name', 'nas');
  type('host', 'nas.lan');
  document.querySelector<HTMLInputElement>('[role="dialog"] input[type="checkbox"]')?.click();
  flushSync();
  type('ssh-username', 'root');
  type('credential-password', 'hunter2');
  return fake;
}

it('DEV-011: Save with a typed SSH password stores it for the new device ID', async () => {
  const fake = await addWithPassword({ SetSshCredential: { setSshCredential: { credentialStored: true } } });
  button('Save').click();
  await settleAll();
  expect(fake.requests.find((request) => request.operation === 'SetSshCredential')?.variables).toEqual({
    deviceId: 'd9',
    credential: { password: 'hunter2' },
  });
  expect(current.goto).toEqual([['/devices/d9', { state: {} }]]);
});

it('Predictable: a credential the hub did not store still opens the device, carrying only the hub message in page state', async () => {
  await addWithPassword({ SetSshCredential: { code: 'UNAVAILABLE', message: 'The vault is locked.' } });
  button('Save').click();
  await settleAll();
  expect(current.goto).toEqual([['/devices/d9', { state: { credentialError: 'The vault is locked.' } }]]);
  expect(JSON.stringify(current.goto)).not.toContain('hunter2');
});

/** Opens a section of the modal's form and turns its protocol on. */
function enable(protocol: 'rdp' | 'vnc'): void {
  document.querySelector<HTMLButtonElement>(`[role="dialog"] [data-section="${protocol}"] button`)?.click();
  flushSync();
  document.querySelector<HTMLInputElement>(`[role="dialog"] [data-section="${protocol}"] input[type="checkbox"]`)?.click();
  flushSync();
}

it('DEV-011, LOG-007: Save with typed RDP and VNC passwords stores each for the new device ID', async () => {
  const fake = await show([device('a')], {
    CreateDevice: { createDevice: DETAIL },
    SetRdpCredential: { setRdpCredential: { credentialStored: true } },
    SetVncCredential: { setVncCredential: { credentialStored: true } },
  });
  await openAddDevice();
  type('name', 'nas');
  type('host', 'nas.lan');
  enable('rdp');
  type('rdp-username', 'admin');
  type('rdp-password', 'rdp-hunter2');
  enable('vnc');
  type('vnc-password', 'vnc-hunter2');
  button('Save').click();
  // The default check and two passwords make more hub round trips than the SSH case.
  await settleAll();
  await settleAll();
  const stored = fake.requests.filter((request) => request.operation.startsWith('Set'));
  expect(stored).toEqual([
    { operation: 'SetRdpCredential', variables: { deviceId: 'd9', credential: { password: 'rdp-hunter2' } } },
    { operation: 'SetVncCredential', variables: { deviceId: 'd9', credential: { password: 'vnc-hunter2' } } },
  ]);
  expect(current.goto).toEqual([['/devices/d9', { state: {} }]]);
});

it('DEV-011, LOG-007: an RDP password the hub did not store still opens the device, and the message never quotes it', async () => {
  await show([device('a')], { CreateDevice: { createDevice: DETAIL }, SetRdpCredential: { code: 'INTERNAL', message: 'refused rdp-hunter2' } });
  await openAddDevice();
  type('name', 'nas');
  type('host', 'nas.lan');
  enable('rdp');
  type('rdp-username', 'admin');
  type('rdp-password', 'rdp-hunter2');
  button('Save').click();
  await settleAll();
  expect(current.goto).toEqual([['/devices/d9', { state: { credentialError: 'The hub did not store the password.' } }]]);
});

it('DEV-013: Test connection in the modal sends the unsaved form values to the hub', async () => {
  const result = { reachable: true, authenticated: true, hostKey: null, hostKeyChanged: false, message: null };
  const fake = await addWithPassword({ TestConnection: { testConnection: result } });
  button('Test connection').click();
  await settleAll();
  const sent = fake.requests.find((request) => request.operation === 'TestConnection')?.variables as { input: Record<string, unknown> };
  expect(sent.input).toMatchObject({ host: 'nas.lan', ssh: { enabled: true, username: 'root' }, credential: { password: 'hunter2' } });
  expect(sent.input).not.toHaveProperty('deviceId');
});

it('WP-13 H-1: when the sites cannot be read the modal stays closed and the page says why', async () => {
  await show([device('a')], { Sites: { code: 'unreachable', message: 'The hub did not answer.' } });
  await openAddDevice();
  expect(dialog()).toBeNull();
  expect(document.querySelector('section > [role="alert"]')?.textContent).toBe('Could not open Add device: The hub did not answer.');
});

it('WP-13 H-1: Escape inside the modal closes the modal only; an open toolbar menu stays open', async () => {
  await show();
  const status = [...document.querySelectorAll<HTMLButtonElement>('.control')].find((node) => node.textContent.includes('Status'));
  status?.click();
  flushSync();
  expect(status?.getAttribute('aria-expanded')).toBe('true');
  await openAddDevice();
  const field = document.querySelector<HTMLElement>('[role="dialog"] input');
  field?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  flushSync();
  await vi.waitFor(() => {
    expect(dialog()).toBeNull();
  });
  expect(status?.getAttribute('aria-expanded')).toBe('true');
});

it('WP-14 H-1: SSH opens a session tab for the device, growing from the button', async () => {
  await show([device('a', { ssh: { enabled: true, hostKey: { fingerprintSha256: 'SHA256:abc' } } })]);
  const ssh = [...document.querySelectorAll('button')].find((node) => node.textContent.trim() === 'SSH');
  expect(ssh).toBeDefined();
  ssh?.click();
  expect(current.opened).toEqual([
    { deviceId: 'a', deviceName: 'Device a', protocol: 'SSH', origin: ssh?.getBoundingClientRect() },
  ]);
  expect(current.goto).toEqual([]);
});
