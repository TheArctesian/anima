// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import type { HubError } from '$lib/hub/hub';
import { fakeHub, picture, settled } from '../vpn.fixture';
import { enrolApi } from './api';
import EnrolModal from './EnrolModal.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const KEY = 'hskey-auth-AbCdEfGhIjKl-0123456789abcdefghijklmnopqrstuvwxyz';
const STEPS = [
  `sudo bash -c 'umask 077; read -rsp "Pre-auth key: " K; echo; printf "%s" "$K" > /run/anima-preauth'`,
  'sudo tailscale up --login-server https://hs.anima.ing --auth-key=file:/run/anima-preauth --accept-routes',
  'sudo rm -f /run/anima-preauth',
];
const MADE = { key: KEY, expiresAt: new Date(Date.now() + 3_600_000).toISOString(), steps: STEPS };
const CONSOLE = ['log', 'info', 'warn', 'error', 'debug'] as const;
const consoleSpies = CONSOLE.map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));

let app: ReturnType<typeof mount> | undefined;
let closed = 0;

beforeEach(() => {
  installAnimations();
  closed = 0;
  for (const spy of consoleSpies) spy.mockClear();
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: () => Promise.resolve() } });
  localStorage.clear();
  sessionStorage.clear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const sites = (waiting = false) => {
  const [home] = picture().sites;
  if (home === undefined) throw new Error('no site in the fixture');
  const router = picture().nodes[1] ?? null;
  return [{ ...home, state: 'NO_ROUTER' as const, router: null, waitingRouter: waiting ? { ...(router as NonNullable<typeof router>), name: 'lab-router', waitingRoutes: ['192.168.1.0/24'] } : null }];
};

async function open(answers: Parameters<typeof fakeHub>[0], siteId: string | null, list = sites()): Promise<{ state: { sites: ReturnType<typeof sites> }; requests: ReturnType<typeof fakeHub>['requests'] }> {
  const fake = fakeHub(answers);
  const state = $state({ sites: list });
  app = mount(EnrolModal, {
    target: document.body,
    props: {
      api: enrolApi(fake.hub),
      siteId,
      onClose: () => { closed += 1; },
      get sites() {
        return state.sites;
      },
    },
  });
  flushSync();
  await settled();
  return { state, requests: fake.requests };
}

const dialog = () => document.querySelector('[role="dialog"], [role="alertdialog"]');
const text = (node: Element | null) => (node?.textContent ?? '').replace(/\s+/g, ' ').trim();
function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((each) => text(each).startsWith(label));
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
}
const keyField = () => document.querySelector<HTMLTextAreaElement>('textarea');
const documentHolds = (secret: string) =>
  document.documentElement.outerHTML.includes(secret) || [...document.querySelectorAll('textarea, input')].some((each) => (each as HTMLInputElement).value.includes(secret));

it('VPN-003: a Linux computer gets a key view with the key, a timer and numbered steps with Copy', async () => {
  await open({ CreateVpnInvite: { createVpnInvite: MADE } }, null);
  button('Linux computer').click();
  await settled();
  expect(keyField()?.value).toBe(KEY);
  expect(dialog()?.querySelector('[role="timer"]')).not.toBeNull();
  const steps = [...document.querySelectorAll('ol[aria-label="Steps"] > li')];
  expect(steps.map((step) => step.querySelector('code')?.textContent)).toEqual(STEPS);
  expect(steps.every((step) => text(step.querySelector('button')) === 'Copy')).toBe(true);
});

it('VP-7: the key is shown once and gone from the page after Close', async () => {
  await open({ CreateVpnInvite: { createVpnInvite: MADE } }, null);
  button('Linux computer').click();
  await settled();
  expect(documentHolds(KEY)).toBe(true);
  const field = keyField();
  button('Close').click();
  expect(field?.value).toBe('');
  expect(documentHolds(KEY)).toBe(false);
  expect(closed).toBe(1);
});

it('VP-7: no step holds the key, and the key reaches no storage, URL or console', async () => {
  await open({ CreateVpnInvite: { createVpnInvite: MADE } }, null);
  button('Linux computer').click();
  await settled();
  for (const step of document.querySelectorAll('ol[aria-label="Steps"] code')) expect(step.textContent).not.toContain(KEY);
  expect(JSON.stringify({ ...localStorage })).not.toContain(KEY);
  expect(JSON.stringify({ ...sessionStorage })).not.toContain(KEY);
  expect(location.href).not.toContain(KEY);
  expect(consoleSpies.flatMap((spy) => spy.mock.calls.flat()).join(' ')).not.toContain(KEY);
  expect(dialog()?.querySelector('[role="status"]')?.textContent).not.toContain(KEY);
});

it('VPN-001: a router key is made for the site and its modal names the site', async () => {
  const { requests } = await open({ CreateRouterKey: { createRouterKey: MADE } }, 'site-home');
  button('Make a router key for Home').click();
  await settled();
  expect(requests).toEqual([{ operation: 'CreateRouterKey', variables: { siteId: 'site-home' } }]);
  expect(keyField()?.value).toBe(KEY);
});

it('VPN-001: when the site has a waiting router the key leaves the page and the modal says the route waits', async () => {
  const { state } = await open({ CreateRouterKey: { createRouterKey: MADE } }, 'site-home');
  button('Make a router key for Home').click();
  await settled();
  state.sites = sites(true);
  flushSync();
  await settled();
  expect(documentHolds(KEY)).toBe(false);
  expect(text(dialog())).toContain('lab-router offers 192.168.1.0/24 and waits for approval.');
});

it('VPN-001: when the key leaves because the router connected, focus moves to the Approve route button', async () => {
  const { state } = await open({ CreateRouterKey: { createRouterKey: MADE } }, 'site-home');
  button('Make a router key for Home').click();
  await settled();
  expect(document.activeElement).toBe(keyField());
  state.sites = sites(true);
  flushSync();
  await settled();
  expect(document.activeElement).toBe(button('Approve route'));
});

it('VPN-003: each copy step is a list item holding its own layout, so the browser numbers it', async () => {
  await open({ CreateVpnInvite: { createVpnInvite: MADE } }, null);
  button('Linux computer').click();
  await settled();
  for (const step of document.querySelectorAll('ol[aria-label="Steps"] > li')) {
    expect(step.querySelector(':scope > .step > code')).not.toBeNull();
    expect(getComputedStyle(step).display).not.toBe('flex');
  }
});

it('vpn.md 6B: the register view warns about an ID from a message and sends the typed ID', async () => {
  const { requests } = await open({ RegisterVpnNode: { registerVpnNode: { name: 'mac', address: '100.64.0.9', keyExpiry: null } } }, null);
  button('Mac or other computer').click();
  await settled();
  expect(text(dialog())).toContain("Type the ID from the new computer's own screen. Never use an ID from a message or a link.");
  const input = document.querySelector<HTMLInputElement>('input[type="text"]');
  if (input === null) throw new Error('no auth ID field');
  input.value = 'hskey-authreq-AbCdEfGhIjKlMnOpQrStUvWx';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  button('Register').click();
  await settled();
  expect(requests).toEqual([{ operation: 'RegisterVpnNode', variables: { authId: 'hskey-authreq-AbCdEfGhIjKlMnOpQrStUvWx' } }]);
  expect(text(dialog())).toContain('mac is on the VPN at 100.64.0.9.');
});

it('VP-7: a hub refusal shows its message in the reserved line and makes no key', async () => {
  const refusal: HubError = { code: 'CONFLICT', message: 'Too many keys in a short time.' };
  await open({ CreateVpnInvite: refusal }, null);
  button('Linux computer').click();
  await settled();
  expect(text(dialog()?.querySelector('[role="alert"]') ?? null)).toBe('Too many keys in a short time.');
  expect(keyField()).toBeNull();
});
