// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { BROWSER } from '$lib/platform/answers.fixture';
import { extensionsApi } from './api';
import { fakeHub, type FakeHub } from './extensions.fixture';
import ExtensionsHarness from './ExtensionsHarness.svelte';
import { ExtensionsStore } from './store.svelte';

vi.mock('$lib/platform/platform', () => ({ capabilities: () => BROWSER }));
vi.mock('$lib/hub/context', () => ({
  useHub: () => ({
    address: { origin: 'https://hub.example' },
    status: { current: { connection: 'connected', lastContactAt: null } },
    request: () => new Promise(() => undefined),
    subscribe: () => () => undefined,
  }),
}));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: FakeHub;

beforeEach(() => {
  installAnimations();
  hub = fakeHub();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

async function open(): Promise<void> {
  app = mount(ExtensionsHarness, { target: document.body, props: { store: new ExtensionsStore(extensionsApi(hub)) } });
  flushSync();
  await settled();
}

const switchOf = (key: string) => document.querySelector<HTMLElement>(`#extension-${key}`);
const isOn = (key: string) => switchOf(key)?.getAttribute('aria-checked') === 'true';
const isDisabled = (key: string) => switchOf(key)?.getAttribute('aria-disabled') === 'true';
const navHrefs = () => [...document.querySelectorAll('nav a')].map((link) => link.getAttribute('href'));

async function flip(key: string): Promise<void> {
  const control = switchOf(key);
  if (control === null) throw new Error(`no switch ${key}`);
  control.click();
  await settled();
}

async function confirm(label: string): Promise<void> {
  const button = [...document.querySelectorAll('button')].find((each) => each.textContent.trim() === label);
  if (button === undefined) throw new Error(`no button ${label}`);
  button.click();
  await settled();
}

it('EXT-001: each extension is a row with its switch, off by default, and a change is sent at once', async () => {
  await open();
  expect(isOn('projects')).toBe(false);
  await flip('projects');
  expect(hub.sent).toEqual(['projects:true']);
  expect(isOn('projects')).toBe(true);
  expect(document.querySelector('button[type="submit"]')).toBeNull();
});

it('EXT-002: the bar shows Projects only while Projects is on, and no longer after it is set off', async () => {
  await open();
  expect(navHrefs()).toEqual(['/devices', '/settings']);
  await flip('projects');
  expect(navHrefs()).toEqual(['/devices', '/projects', '/settings']);
  await flip('projects');
  expect(hub.sent).toEqual(['projects:true', 'projects:false']);
  expect(navHrefs()).toEqual(['/devices', '/settings']);
});

it('EXT-011: the "also sets Secrets on" sentence shows before anything is sent, and the change waits for the yes', async () => {
  await open();
  await flip('model_gateway');
  expect(document.body.textContent).toContain('Model gateway also sets Secrets on');
  expect(hub.sent).toEqual([]);
  expect(isOn('model_gateway')).toBe(false);
  await confirm('Set on');
  expect(hub.sent).toEqual(['model_gateway:true']);
  expect(isOn('secrets')).toBe(true);
});

it('EXT-011: cancelling the sentence sends nothing', async () => {
  await open();
  await flip('model_gateway');
  await confirm('Cancel');
  expect(hub.sent).toEqual([]);
  expect(isOn('model_gateway')).toBe(false);
});

it('EXT-012: a needed extension has a disabled switch that names the extension that needs it', async () => {
  await open();
  await flip('model_gateway');
  await confirm('Set on');
  expect(isDisabled('secrets')).toBe(true);
  expect(document.querySelector('#extension-secrets-hint')?.textContent).toContain('Model gateway is on and needs it');
  expect(isDisabled('model_gateway')).toBe(false);
});

it('EXT-001: an extension that is not built has no live switch and says so', async () => {
  await open();
  expect(switchOf('map')).toBeNull();
  expect(document.querySelector('#extension-map-hint')?.textContent).toBe('Not built yet');
});

it('EXT-012: a refused change puts the switch back and shows the hub words', async () => {
  await open();
  hub.refuse = { code: 'CONFLICT', message: 'Model gateway is on and needs Secrets. Set Model gateway off first.' };
  await flip('projects');
  expect(hub.sent).toEqual(['projects:true']);
  expect(isOn('projects')).toBe(false);
  const alerts = [...document.querySelectorAll('[role="alert"]')].map((each) => each.textContent).filter((text) => text !== '');
  expect(alerts).toEqual([hub.refuse.message]);
  expect(navHrefs()).toEqual(['/devices', '/settings']);
});
