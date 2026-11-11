// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import type { HubError } from '$lib/hub/hub';
import { fakeHub, node, NOW, picture, ROUTER, settled, STRANGER } from './vpn.fixture';
import VpnSection from './VpnSection.svelte';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(answers: Parameters<typeof fakeHub>[0]): Promise<HTMLElement> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(VpnSection, { target });
  flushSync();
  await settled();
  return target;
}

const text = (node: Element | null | undefined) => (node?.textContent ?? '').replace(/\s+/g, ' ').trim();
const site = (target: HTMLElement, name: string) => target.querySelector<HTMLElement>(`ul[aria-label="Sites"] > li[aria-label="${name}"]`);
function home() {
  const first = picture().sites[0];
  if (first === undefined) throw new Error('no site in the fixture');
  return first;
}
const FAILED: HubError = { code: 'unreachable', message: 'secret hub text' } as HubError;

it('SET-003: with no proxy the section says VPN status is off', async () => {
  const target = await open({ Vpn: { vpn: null } });
  expect(text(target)).toContain('VPN status is not available. The hub has no LocalAPI proxy (ANIMA_TAILSCALE_LOCALAPI)');
  expect(target.querySelector('ul')).toBeNull();
});

it('SET-003, VPN-009: each site shows its state and path as text', async () => {
  const target = await open({ Vpn: { vpn: picture() } });
  const row = site(target, 'Home');
  expect(text(row)).toContain('192.168.1.0/24');
  expect(text(row?.querySelector('.state'))).toBe('Serving');
  expect(text(row?.querySelector('.path'))).toBe('Direct');
  expect(row?.querySelector('.dot')?.getAttribute('aria-hidden')).toBe('true');
});

it('VPN-009: a relayed site with its router offline says so in words', async () => {
  const sites = [{ ...home(), state: 'OFFLINE' as const, path: 'RELAYED' as const, relayRegion: 'sfo', hasPingCheck: true }];
  const target = await open({ Vpn: { vpn: picture({ sites }) } });
  expect(text(site(target, 'Home'))).toContain('Router offline');
  expect(text(site(target, 'Home'))).toContain('Relayed (sfo)');
});

it('VPN-009: a serving site without a ping check offers one, linked to the router device Checks tab', async () => {
  const target = await open({ Vpn: { vpn: picture() } });
  const link = site(target, 'Home')?.querySelector('a');
  expect(text(link)).toBe('Add a ping check');
  expect(link?.getAttribute('href')).toBe('/devices/00000000-0000-4000-8000-000000000002?tab=checks');
});

it('VPN-009: a serving site that has a ping check offers none', async () => {
  const sites = [{ ...home(), hasPingCheck: true }];
  const target = await open({ Vpn: { vpn: picture({ sites }) } });
  expect(target.querySelector('a')).toBeNull();
});

it('VPN-009: a router that is not a device in Anima says so instead of offering a check', async () => {
  const sites = [{ ...home(), router: { ...ROUTER, device: null, inAnima: false } }];
  const target = await open({ Vpn: { vpn: picture({ sites }) } });
  expect(target.querySelector('a')).toBeNull();
  expect(text(site(target, 'Home'))).toContain('The router is not a device in Anima');
});

it('VPN-010: keys are listed with their expiry text', async () => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(NOW);
  try {
    const target = await open({ Vpn: { vpn: picture() } });
    const rows = [...target.querySelectorAll('ul[aria-label="VPN nodes"] > li')];
    expect(rows.map((row) => row.getAttribute('aria-label'))).toEqual(['hub', 'stranger', 'mac', 'router-home']);
    expect(rows.map((row) => text(row.querySelector('.key')))).toEqual(['Does not expire', 'Expires in 9 days', expect.stringMatching(/^Expires on /), 'Does not expire']);
  } finally {
    vi.useRealTimers();
  }
});

it('VPN-011: a node not in Anima is marked and listed before the Anima nodes', async () => {
  const target = await open({ Vpn: { vpn: picture({ nodes: [node(), STRANGER] }) } });
  const rows = [...target.querySelectorAll('ul[aria-label="VPN nodes"] > li')];
  expect(rows.map((row) => row.getAttribute('aria-label'))).toEqual(['hub', 'stranger', 'mac']);
  expect(text(rows[1])).toContain('Not in Anima');
});

it('SET-003: a new picture from vpnChanged replaces the shown one, and the hub is asked for one subscription', async () => {
  const target = await open({ Vpn: { vpn: picture() } });
  expect(hub.subscriptions.map((each) => each.operation)).toEqual(['VpnChanged']);
  const sites = [{ ...home(), site: { id: 'site-away', name: 'Away', subnet: '10.9.0.0/24' }, hasPingCheck: true }];
  hub.push('VpnChanged', { vpnChanged: picture({ sites }) });
  await settled();
  expect(site(target, 'Away')).not.toBeNull();
});

it('SET-003: a picture that is not fresh shows when it was last read, as a status', async () => {
  const target = await open({ Vpn: { vpn: picture({ fresh: false }) } });
  const line = target.querySelector('[role="status"]');
  expect(text(line)).toMatch(/^Last read .+ ago$/);
});

it('SET-003: a failed load shows a fixed message, not the hub text, and Retry loads again', async () => {
  const answers: Record<string, unknown> = { Vpn: FAILED };
  const target = await open(answers);
  expect(text(target.querySelector('[role="alert"]'))).toBe('Could not load the VPN status.');
  expect(target.textContent).not.toContain('secret hub text');
  expect(hub.subscriptions).toHaveLength(0);
  answers.Vpn = { vpn: picture() };
  [...target.querySelectorAll('button')].find((each) => text(each) === 'Retry')?.click();
  await settled();
  expect(hub.requests.filter((each) => each.operation === 'Vpn')).toHaveLength(2);
  expect(site(target, 'Home')).not.toBeNull();
  expect(target.querySelector('[role="alert"]')).toBeNull();
});

it('MOT-007: while loading the rows are placeholders that assistive technology skips', async () => {
  hub = fakeHub();
  hub.hub.request = (() => new Promise(() => undefined)) as typeof hub.hub.request;
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(VpnSection, { target });
  flushSync();
  const waiting = target.querySelector('.waiting');
  expect(waiting?.getAttribute('aria-hidden')).toBe('true');
  expect(waiting?.hasAttribute('inert')).toBe(true);
  expect(waiting?.querySelectorAll('li')).toHaveLength(2);
});

it('SET-003: leaving the section stops listening', async () => {
  await open({ Vpn: { vpn: picture() } });
  expect(hub.listeners('VpnChanged')).toBe(1);
  await unmount(app as ReturnType<typeof mount>);
  app = undefined;
  expect(hub.listeners('VpnChanged')).toBe(0);
});
