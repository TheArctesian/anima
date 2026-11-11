// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { node, picture } from '$lib/vpn/vpn.fixture';
import Page from '../+page.svelte';
import { parseDeviceId } from '../device-id';
import { ID, OTHER_ID, answers, route } from '../page.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({ useSessions: () => ({ openSession: () => undefined }) }));
vi.mock('$app/navigation', () => ({ goto: () => Promise.resolve() }));
vi.mock('$app/state', async () => {
  const { route: shown } = await import('../page.fixture.svelte');
  return {
    page: {
      get url() {
        return new URL(`/devices/${shown.param}${shown.search}`, 'http://localhost');
      },
      get params() {
        return { id: shown.param };
      },
      get state() {
        return shown.state;
      },
    },
  };
});

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function show(vpn: unknown): Promise<ReturnType<typeof fakeHub>> {
  route.param = ID;
  route.search = '';
  route.state = {};
  const fake = fakeHub(answers({ Vpn: { vpn } }));
  current.hub = fake.hub;
  app = mount(Page, { target: document.body, props: { data: { id: parseDeviceId(ID) } } });
  for (let turn = 0; turn < 8; turn += 1) await tick();
  flushSync();
  return fake;
}

const vpnCard = () => [...document.querySelectorAll('section')].find((one) => one.querySelector('h2')?.textContent.endsWith('VPN'));

it('VPN-005: the page reads the VPN once and the Overview shows the card for this device\'s node', async () => {
  const mine = node({ address: '100.64.0.9', device: { id: ID, name: 'nas' } });
  const other = node({ name: 'other', address: '100.64.0.8', device: { id: OTHER_ID, name: 'other' } });
  const fake = await show(picture({ nodes: [other, mine] }));
  expect(fake.requests.filter((request) => request.operation === 'Vpn')).toHaveLength(1);
  expect(fake.subscriptions.filter((request) => request.operation === 'VpnChanged')).toHaveLength(1);
  expect(vpnCard()?.textContent).toContain('100.64.0.9');
  expect(vpnCard()?.textContent).not.toContain('100.64.0.8');
});

it('VPN-005: a device with no node in the VPN picture, or no VPN status, shows no VPN card', async () => {
  await show(picture({ nodes: [node({ device: { id: OTHER_ID, name: 'other' } })] }));
  expect(vpnCard()).toBeUndefined();
  await unmount(app as ReturnType<typeof mount>);
  document.body.replaceChildren();
  await show(null);
  expect(vpnCard()).toBeUndefined();
  expect(document.querySelector('h1')).not.toBeNull();
});
