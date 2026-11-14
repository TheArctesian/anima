// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../../motion/engine.fixture';
import type { CardDevice } from '../api';
import { fakeHub } from '../hub.fixture';
import { DEFAULT_QUERY, type ListQuery } from '../query';
import DeviceScreen from '../screen/DeviceScreen.svelte';
import { CARDS, device, devicesAnswer, url } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

const KEYED = { enabled: true, hostKey: { fingerprintSha256: 'SHA256:abc' } };

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
  query: ListQuery = CARDS,
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
      onConnect: (target: CardDevice) => connected.push(target.id),
    },
  });
  await settle();
}


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

it('6.1 card: SSH shows only when enabled with an accepted host key, and passes the device to onConnect', async () => {
  await show([device('a', { ssh: KEYED }), device('b'), device('c', { ssh: { enabled: true, hostKey: null } })]);
  const buttons = [...document.querySelectorAll('article button')];
  expect(buttons).toHaveLength(1);
  (buttons[0] as HTMLButtonElement).click();
  expect(connected).toEqual(['a']);
});

it('6.1 card: the name and the pill carry data-device-id (H-1)', async () => {
  await show([device('a')]);
  expect(document.querySelectorAll('[data-device-id="a"]')).toHaveLength(2);
});

it.each([
  ['grid', 'article a'],
  ['table', 'tbody th a'],
] as const)('6.1 card: the %s name link encodes an id with / ? and #', async (view, selector) => {
  await show([device('a/b?c#d')], { ...CARDS, view });
  expect(document.querySelector(selector)?.getAttribute('href')).toBe('/devices/a%2Fb%3Fc%23d');
});

it.each([
  ['grid', 'article'],
  ['table', 'tbody tr'],
] as const)('UI-015: each %s device shows its OS logo, named in words', async (view, selector) => {
  await show([device('a', { os: 'MACOS' }), device('b', { os: 'WINDOWS' })], { ...CARDS, view });
  const names = [...document.querySelectorAll(selector)].map((row) => row.querySelector('[role="img"][aria-label]:not(.bar)')?.getAttribute('aria-label'));
  expect(names).toEqual(['macOS', 'Windows']);
});

it.each([
  ['grid', 'article'],
  ['table', 'tbody tr'],
] as const)('DEV-016: a %s Linux device shows its distribution logo once the hub has read it, else the Linux logo', async (view, selector) => {
  await show(
    [
      device('a', { os: 'LINUX', distribution: 'pop' }),
      device('b', { os: 'LINUX', distribution: null }),
      device('c', { os: 'LINUX', distribution: 'someos' }),
    ],
    { ...CARDS, view },
  );
  const names = [...document.querySelectorAll(selector)].map((row) => row.querySelector('[role="img"][aria-label]:not(.bar)')?.getAttribute('aria-label'));
  expect(names).toEqual(['Pop!_OS', 'Linux', 'Linux']);
});

it('6.1 card: in the table SSH shows only when enabled with an accepted host key, and calls onConnect', async () => {
  await show([device('a', { ssh: KEYED }), device('b'), device('c', { ssh: { enabled: true, hostKey: null } })], {
    ...CARDS,
    view: 'table',
  });
  const buttons = [...document.querySelectorAll('tbody button')];
  expect(buttons).toHaveLength(1);
  (buttons[0] as HTMLButtonElement).click();
  expect(connected).toEqual(['a']);
});

it('6.1 card: in the table the IP shows only next to a host name, not for an IP host or an empty ip', async () => {
  await show(
    [device('a', { ip: '10.0.0.5' }), device('b', { host: '10.0.0.9', ip: '10.0.0.9' }), device('c', { ip: '' })],
    { ...CARDS, view: 'table' },
  );
  const rows = [...document.querySelectorAll('tbody tr')];
  expect(rows.map((row) => row.querySelectorAll('.ip').length)).toEqual([1, 0, 0]);
});

it('6.1 card: in the grid an empty ip shows no IP', async () => {
  await show([device('c', { ip: '' })]);
  expect(document.querySelectorAll('article .ip')).toHaveLength(0);
});

it('WebKit: every card and table control has tabindex 0, or Tab skips it (NAV-005)', async () => {
  for (const view of ['grid', 'table'] as const) {
    await show([device('a', { ssh: KEYED })], { ...CARDS, view });
    const controls = [...document.querySelectorAll('button, a[href]')];
    expect(controls.length).toBeGreaterThan(6);
    expect(controls.filter((node) => node.getAttribute('tabindex') !== '0')).toEqual([]);
    await unmount(app as object);
    document.body.innerHTML = '';
  }
});

it('WebKit: the empty-state, Load more and Retry buttons have tabindex 0', async () => {
  await show([], DEFAULT_QUERY);
  expect(document.querySelector('.empty button')?.getAttribute('tabindex')).toBe('0');
  await unmount(app as object);
  document.body.innerHTML = '';
  await show([device('a')], DEFAULT_QUERY, { Devices: devicesAnswer([device('a')], 'c1') });
  expect(button('Load more').getAttribute('tabindex')).toBe('0');
  await unmount(app as object);
  document.body.innerHTML = '';
  await show([], DEFAULT_QUERY, { Devices: { code: 'unreachable', message: 'down' } });
  expect(button('Retry').getAttribute('tabindex')).toBe('0');
});
