// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Page from '../../../routes/devices/+page.svelte';
import { button } from '../form/form/form.fixture';
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
  url.query = { view: 'grid', name: '', statuses: [], tags: [], site: null, sort: 'NAME' };
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
  const list = document.querySelector<HTMLButtonElement>('.views button[aria-label="List"]');
  list?.click();
  expect(current.goto).toEqual([['/devices?sort=NAME', { replaceState: true, keepFocus: true, noScroll: true }]]);
});

it('DEV-003: the URL query is what the page shows (Back and Forward restore the view)', async () => {
  await show();
  url.query = { ...url.query, view: 'table' };
  flushSync();
  expect(document.querySelector('table')).not.toBeNull();
  url.query = { ...url.query, view: 'grid' };
  flushSync();
  expect(document.querySelector('ul.grid')).not.toBeNull();
  // The list fades out under the grid, then leaves (5.7.3).
  await vi.waitFor(() => {
    expect(document.querySelector('table')).toBeNull();
  });
});

const addButton = () => [...document.querySelectorAll('button')].find((node) => node.textContent.includes('Add device'));
const dialog = () => document.querySelector('[role="dialog"]');

async function openAddDevice(): Promise<void> {
  addButton()?.click();
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
}

it('WP-13 H-1, DEV-010: Add device opens enrollment over the list', async () => {
  await show();
  await openAddDevice();
  expect(dialog()?.textContent).toContain('Add device');
  expect(current.goto).toEqual([]);
});

it('Add device gives the other device a one-use QR without making an inventory entry', async () => {
  const invite = 'anima-invite.1.ZGV2aWNl.anima-link.1.example';
  const fake = await show([device('a')], { CreateLinkInvite: { createLinkInvite: { invite, expiresAt: '2100-01-01T00:00:00Z' } } });
  await openAddDevice();
  expect(dialog()?.querySelector('input')).toBeNull();
  button('Enroll another device').click();
  flushSync();
  const name = dialog()?.querySelector('input');
  expect(name).toBeDefined();
  if (name === null || name === undefined) throw new Error('Missing device name');
  name.value = 'Kitchen tablet';
  name.dispatchEvent(new Event('input', { bubbles: true }));
  button('Make invite').click();
  await vi.waitFor(() => {
    flushSync();
    expect(dialog()?.querySelector('[role="img"][aria-label="Invite for Kitchen tablet"]')).not.toBeNull();
  });
  expect(fake.requests.filter(({ operation }) => operation === 'CreateLinkInvite')).toEqual([
    { operation: 'CreateLinkInvite', variables: { name: 'Kitchen tablet' } },
  ]);
  expect(fake.requests.some(({ operation }) => operation === 'CreateDevice')).toBe(false);
});

it('WP-13 H-1: Escape inside the modal closes the modal only; an open toolbar menu stays open', async () => {
  await show();
  const status = [...document.querySelectorAll<HTMLButtonElement>('.control')].find((node) => node.textContent.includes('Tags'));
  status?.click();
  flushSync();
  expect(status?.getAttribute('aria-expanded')).toBe('true');
  await openAddDevice();
  const field = document.querySelector<HTMLElement>('[role="dialog"] button');
  field?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  flushSync();
  await vi.waitFor(() => {
    expect(dialog()).toBeNull();
  });
  expect(status?.getAttribute('aria-expanded')).toBe('true');
});

it('WP-14 H-1: SSH opens a session tab for the device, growing from the button', async () => {
  await show([device('a', { ssh: { enabled: true, hostKey: { fingerprintSha256: 'SHA256:abc' } } })]);
  const ssh = document.querySelector<HTMLButtonElement>('button[aria-label="SSH to Device a"]') ?? undefined;
  expect(ssh).toBeDefined();
  ssh?.click();
  expect(current.opened).toEqual([
    { deviceId: 'a', deviceName: 'Device a', os: 'OTHER', protocol: 'SSH', origin: ssh?.getBoundingClientRect() },
  ]);
  expect(current.goto).toEqual([]);
});

it('VPN-002: the device list asks the hub with siteId', async () => {
  url.query = { ...url.query, site: 's1' };
  const fake = await show();
  const asked = fake.requests.filter((request) => request.operation === 'Devices');
  expect(asked.at(-1)?.variables).toMatchObject({ filter: { siteId: 's1' } });
});

it('VPN-002: Clear filters clears the site', async () => {
  url.query = { ...url.query, site: 's1' };
  await show([]);
  button('Clear filters').click();
  expect(current.goto.at(-1)?.[0]).toBe('/devices?view=grid&sort=NAME');
});
