// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../../motion/engine.fixture';
import { fakeHub } from '../hub.fixture';
import { DEFAULT_QUERY, listVariables, type ListQuery } from '../query';
import DeviceScreen from './DeviceScreen.svelte';
import { device, devicesAnswer, url } from './screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;
let fake: ReturnType<typeof fakeHub>;
const added: string[] = [];
const connected: string[] = [];

/** Lets the pending promises settle and the DOM update. */
async function settle(): Promise<void> {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
}

async function show(
  devices = [device('a'), device('b')],
  query: ListQuery = DEFAULT_QUERY,
  extra: Record<string, unknown> = {},
): Promise<void> {
  fake = fakeHub({ Devices: devicesAnswer(devices), Tags: { tags: ['lab', 'prod'] }, ...extra });
  current.hub = fake.hub;
  url.query = query;
  app = mount(DeviceScreen, {
    target: document.body,
    props: {
      get query() {
        return url.query;
      },
      onQueryChange: (next: ListQuery) => {
        url.writes.push(next);
        url.query = next;
      },
      onAddDevice: () => added.push('add'),
      onConnect: (id: string) => connected.push(id),
    },
  });
  await settle();
}

const names = () => [...document.querySelectorAll('article h2')].map((node) => node.textContent.trim());
const pills = () => [...document.querySelectorAll('.pill')].map((node) => node.textContent.trim());
const button = (text: string) =>
  [...document.querySelectorAll('button')].find((node) => node.textContent.includes(text)) as HTMLButtonElement;

beforeEach(() => {
  runs.length = 0;
  added.length = 0;
  connected.length = 0;
  url.writes = [];
  preference.reduced = false;
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.useRealTimers();
});

it('DEV-001: 200 devices from one request render 200 cards', async () => {
  await show(Array.from({ length: 200 }, (_, index) => device(String(index).padStart(3, '0'))));
  expect(document.querySelectorAll('article')).toHaveLength(200);
  expect(fake.requests.filter((request) => request.operation === 'Devices')).toHaveLength(1);
  expect(fake.requests[0]?.variables).toMatchObject({ first: 200 });
});

it('DEV-001: Load more asks for the next page with the end cursor and adds its cards', async () => {
  await show([device('a')], DEFAULT_QUERY, { Devices: devicesAnswer([device('a')], 'cursor-1') });
  button('Load more').click();
  await settle();
  const requests = fake.requests.filter((request) => request.operation === 'Devices');
  expect(requests).toHaveLength(2);
  expect(requests[1]?.variables).toEqual(listVariables(DEFAULT_QUERY, 'cursor-1'));
});

it('DEV-002: a deviceStatusChanged push changes the pill without a new list request', async () => {
  await show();
  fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'a', status: 'DOWN', at: 'now' } });
  await settle();
  expect(pills()).toContain('Down');
  expect(fake.requests.filter((request) => request.operation === 'Devices')).toHaveLength(1);
});

it('DEV-002: the table row changes with the push too', async () => {
  await show([device('a'), device('b')], { ...DEFAULT_QUERY, view: 'table' });
  fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'b', status: 'DEGRADED', at: 'now' } });
  await settle();
  const row = [...document.querySelectorAll('tbody tr')].find((node) => node.textContent.includes('Device b'));
  expect(row?.querySelector('.pill')?.textContent.trim()).toBe('Degraded');
});

it('DEV-002: a heartbeatAdded push adds a beat to the right device', async () => {
  await show();
  const beat = { checkId: 'c1', deviceId: 'b', heartbeat: { at: '2026-11-04T10:00:00Z', status: 'DOWN', responseTimeMs: null } };
  fake.push('HeartbeatAdded', { heartbeatAdded: beat });
  await settle();
  const labels = [...document.querySelectorAll('[role="img"]')].map((node) => node.getAttribute('aria-label'));
  expect(labels).toEqual(['No heartbeats yet', '1 heartbeat, latest down']);
});

it('DEV-003: the status filter writes the query and the next request carries it', async () => {
  await show();
  expect(document.querySelectorAll('input[type=checkbox]')).toHaveLength(0);
  button('Status').click();
  await settle();
  document.querySelectorAll<HTMLInputElement>('input[type=checkbox]')[1]?.click();
  await settle();
  expect(url.writes.at(-1)?.statuses).toEqual(['DOWN']);
  const requests = fake.requests.filter((request) => request.operation === 'Devices');
  expect(requests.at(-1)?.variables).toMatchObject({ filter: { status: ['DOWN'] } });
});

it('DEV-003: the tag filter lists the hub tags and writes the chosen one', async () => {
  await show();
  button('Tags').click();
  await settle();
  const labels = [...document.querySelectorAll('.option')].map((node) => node.textContent.trim());
  expect(labels).toEqual(['lab', 'prod']);
  document.querySelectorAll<HTMLInputElement>('input[type=checkbox]')[1]?.click();
  await settle();
  expect(url.writes.at(-1)?.tags).toEqual(['prod']);
  expect(fake.requests.at(-1)?.variables).toMatchObject({ filter: { tags: ['prod'] } });
});

it('DEV-003: typing a name writes the URL 200 ms after the last keystroke', async () => {
  await show();
  vi.useFakeTimers();
  const input = document.querySelector<HTMLInputElement>('input[type=search]');
  if (input === null) throw new Error('no search');
  for (const text of ['n', 'ne', 'new']) {
    input.value = text;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    vi.advanceTimersByTime(150);
  }
  expect(url.writes).toEqual([]);
  vi.advanceTimersByTime(50);
  flushSync();
  expect(url.writes.map((query) => query.name)).toEqual(['new']);
  await vi.advanceTimersByTimeAsync(0);
  expect(fake.requests.at(-1)?.variables).toMatchObject({ filter: { name: 'new' } });
});

it('DEV-003: a push to a status outside the status filter removes the card', async () => {
  await show([device('a'), device('b')], { ...DEFAULT_QUERY, statuses: ['UP'] });
  fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'a', status: 'DOWN', at: 'now' } });
  await settle();
  await vi.waitFor(() => {
    expect(names()).toEqual(['Device b']);
  });
});

it('DEV-003: a push for an unknown device under a status filter fetches the list again', async () => {
  await show([device('a')], { ...DEFAULT_QUERY, statuses: ['DOWN'] });
  fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'zz', status: 'DOWN', at: 'now' } });
  await settle();
  expect(fake.requests.filter((request) => request.operation === 'Devices')).toHaveLength(2);
});

it('DEV-003: a push for an unknown device with no status filter does not fetch', async () => {
  await show([device('a')]);
  fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'zz', status: 'DOWN', at: 'now' } });
  await settle();
  expect(fake.requests.filter((request) => request.operation === 'Devices')).toHaveLength(1);
});

it.each([
  ['Status', 'STATUS'],
  ['Uptime', 'UPTIME'],
  ['Name', 'NAME'],
] as const)('DEV-004: the %s sort writes the URL and sends sort %s', async (label, sort) => {
  await show([device('a')], { ...DEFAULT_QUERY, sort: sort === 'NAME' ? 'STATUS' : 'NAME' });
  button('Sort').click();
  await settle();
  const radio = [...document.querySelectorAll<HTMLInputElement>('.option')].find((node) => node.textContent.trim() === label);
  radio?.querySelector('input')?.click();
  await settle();
  expect(url.writes.at(-1)?.sort).toBe(sort);
  expect(fake.requests.at(-1)?.variables).toMatchObject({ sort });
});

it('DEV-004: under STATUS, a device that turns Down moves first', async () => {
  await show([device('a'), device('b'), device('c')], { ...DEFAULT_QUERY, sort: 'STATUS' });
  fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'c', status: 'DOWN', at: 'now' } });
  await settle();
  expect(names()).toEqual(['Device c', 'Device a', 'Device b']);
});

it('DEV-004: with reduced motion the new order is instant, without reduced motion the moved cards spring', async () => {
  const place = vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
    const top = this.tagName === 'LI' ? [...(this.parentElement?.children ?? [])].indexOf(this) * 100 : 0;
    return { left: 0, right: 0, top, bottom: top, width: 0, height: 0, x: 0, y: top, toJSON: () => ({}) };
  });
  for (const reduced of [true, false]) {
    runs.length = 0;
    preference.reduced = reduced;
    await show([device('a'), device('b'), device('c')], { ...DEFAULT_QUERY, sort: 'STATUS' });
    fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'c', status: 'DOWN', at: 'now' } });
    await settle();
    expect(names()[0]).toBe('Device c');
    expect(runs.some((run) => run.options['type'] === 'spring')).toBe(!reduced);
    await unmount(app as object);
    document.body.innerHTML = '';
  }
  place.mockRestore();
});

it('DEV-005: no devices and no filter shows "No devices yet" and an Add device button', async () => {
  await show([]);
  expect(document.body.textContent).toContain('No devices yet');
  const empty = document.querySelector('.empty');
  empty?.querySelector('button')?.click();
  expect(added).toEqual(['add']);
});

it('DEV-005: a filter with no result shows "No devices match" and Clear filters', async () => {
  await show([], { ...DEFAULT_QUERY, name: 'zzz', sort: 'STATUS' });
  expect(document.body.textContent).toContain('No devices match');
  expect(document.body.textContent).not.toContain('No devices yet');
  button('Clear filters').click();
  await settle();
  expect(url.writes.at(-1)).toEqual({ ...DEFAULT_QUERY, sort: 'STATUS' });
});

it('6.1 card: shows name link, host, IP for a named host, uptime and a 50-cell bar', async () => {
  await show([device('a', { ip: '10.0.0.5', uptime: null }), device('b', { host: '10.0.0.9', ip: '10.0.0.9' })]);
  const cards = [...document.querySelectorAll('article')];
  expect(cards[0]?.textContent).toContain('a.lan');
  expect(cards[0]?.textContent).toContain('10.0.0.5');
  expect(cards[0]?.textContent).toContain('—');
  expect(cards[0]?.querySelectorAll('.cell')).toHaveLength(50);
  expect(cards[0]?.querySelector('a')?.getAttribute('href')).toBe('/devices/a');
  expect(cards[1]?.querySelectorAll('.ip')).toHaveLength(0);
});

it('6.1 card: SSH shows only when enabled and passes the device id to onConnect', async () => {
  await show([device('a', { ssh: { enabled: true } }), device('b')]);
  const buttons = [...document.querySelectorAll('article button')];
  expect(buttons).toHaveLength(1);
  (buttons[0] as HTMLButtonElement).click();
  expect(connected).toEqual(['a']);
});

it('6.1 card: the name and the pill carry data-device-id (H-1)', async () => {
  await show([device('a')]);
  expect(document.querySelectorAll('[data-device-id="a"]')).toHaveLength(2);
});

it('DEV-001: the cards are a list, and the table has column headers', async () => {
  await show();
  expect(document.querySelectorAll('ul[aria-label="Devices"] > li')).toHaveLength(2);
  url.query = { ...DEFAULT_QUERY, view: 'table' };
  await settle();
  expect([...document.querySelectorAll('thead th')].map((node) => node.textContent)).toEqual([
    'Name',
    'Status',
    'Host',
    'Uptime, 24 h',
    'Heartbeats',
    'SSH',
  ]);
  expect(fake.requests.filter((request) => request.operation === 'Devices')).toHaveLength(1);
});

it('K7: a failed load shows the error text and Retry loads again', async () => {
  await show([], DEFAULT_QUERY, { Devices: { code: 'unreachable', message: 'The hub did not answer. Check the VPN client.' } });
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('Check the VPN client');
  button('Retry').click();
  await settle();
  expect(fake.requests.filter((request) => request.operation === 'Devices')).toHaveLength(2);
});

it('DEV-002: both subscriptions stop when the page goes away', async () => {
  await show();
  expect(fake.listeners('DeviceStatusChanged')).toBe(1);
  expect(fake.listeners('HeartbeatAdded')).toBe(1);
  await unmount(app as object);
  app = undefined;
  expect(fake.listeners('DeviceStatusChanged')).toBe(0);
  expect(fake.listeners('HeartbeatAdded')).toBe(0);
});
