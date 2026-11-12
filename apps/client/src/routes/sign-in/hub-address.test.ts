// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub, ok } from '$lib/access/access.fixture';
import type { Hub } from '$lib/hub/hub';
import Page from './+page.svelte';
import { input, pasteInto, problemOf, submit, type } from './page.fixture';

const mocks = vi.hoisted(() => ({
  hub: undefined as unknown,
  created: [] as { origin: string }[],
  writeAccessToken: vi.fn(),
  goToDevices: vi.fn(),
  reloadClient: vi.fn(),
  loadClientAt: vi.fn(),
  desktop: true,
}));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/hub/hub', async (importOriginal) => {
  const real = await importOriginal<typeof import('$lib/hub/hub')>();
  const { fakeHub: fake, ok: answer } = await import('$lib/access/access.fixture');
  return {
    ...real,
    // The hub for an address typed on the page: it answers Login at once and is recorded.
    createHub: (options: { address: { origin: string } }) => {
      mocks.created.push(options.address);
      return { ...fake({ Login: answer({ login: { token: 'tok-moved' } }) }).hub, address: options.address };
    },
  };
});
vi.mock('$lib/desktop/token', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/token')>()),
  writeAccessToken: mocks.writeAccessToken,
}));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER, DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => (mocks.desktop ? DESKTOP_MAC : BROWSER) };
});
vi.mock('$lib/access/navigation', () => ({ goToDevices: mocks.goToDevices, reloadClient: mocks.reloadClient, loadClientAt: mocks.loadClientAt }));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

function open(): HTMLElement {
  hub = fakeHub({ Login: ok({ login: { token: 'tok-here' } }) });
  mocks.hub = hub.hub satisfies Hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

function change(target: HTMLElement): HTMLButtonElement {
  const button = [...target.querySelectorAll('button')].find((each) => each.textContent.trim() === 'Change');
  if (button === undefined) throw new Error('no Change button');
  return button;
}

async function reveal(target: HTMLElement): Promise<void> {
  change(target).click();
  await vi.waitFor(() => {
    flushSync();
    expect(target.querySelector('#hub-address')).not.toBeNull();
  });
}

function press(element: HTMLElement, key: string): void {
  element.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  flushSync();
}

beforeEach(() => {
  mocks.created.length = 0;
  mocks.writeAccessToken.mockReset().mockResolvedValue(undefined);
  mocks.goToDevices.mockReset();
  mocks.reloadClient.mockReset();
  mocks.loadClientAt.mockReset();
  mocks.desktop = true;
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
});

it('NAV-009: the hub is one line with its host name and a Change button; there is no address field and no Save address button', () => {
  const target = open();
  expect(target.textContent).toContain('Hub: hub.example.org');
  expect(target.querySelector('#hub-address')).toBeNull();
  expect(target.textContent).not.toContain('Save address');
  expect(target.querySelectorAll('button')).toHaveLength(2);
});

it('NAV-009: Change opens the address field in place with the current address, and focus moves into it', async () => {
  const target = open();
  await reveal(target);
  expect(input(target, 'hub-address').value).toBe(hub.hub.address.origin);
  expect(document.activeElement).toBe(input(target, 'hub-address'));
});

it('NAV-009: Escape closes the field, restores the address and puts focus back on Change', async () => {
  const target = open();
  await reveal(target);
  type(target, 'hub-address', 'other.example.org');
  press(input(target, 'hub-address'), 'Escape');
  await vi.waitFor(() => {
    flushSync();
    expect(target.querySelector('#hub-address')).toBeNull();
  });
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  expect(mocks.reloadClient).not.toHaveBeenCalled();
  await reveal(target);
  expect(input(target, 'hub-address').value).toBe(hub.hub.address.origin);
});

it('NAV-009, CLI-002: Enter in the address field stores a valid address and reloads the client', async () => {
  const target = open();
  await reveal(target);
  type(target, 'hub-address', 'hub2.example.org');
  await submit(target, 1);
  expect(localStorage.getItem('anima.hubAddress')).toBe('https://hub2.example.org');
  expect(mocks.reloadClient).toHaveBeenCalledOnce();
  expect(hub.sent).toEqual([]);
});

it('CLI-002: an invalid address stores nothing, does not reload and shows its error under the field without the input', async () => {
  const target = open();
  await reveal(target);
  type(target, 'hub-address', 'http://user:secretpw@hub.example.org');
  await submit(target, 1);
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  expect(mocks.reloadClient).not.toHaveBeenCalled();
  const problem = problemOf(target, 'hub-address');
  expect(problem).not.toBe('');
  expect(problem).not.toContain('secretpw');
  expect(input(target, 'hub-address').getAttribute('aria-invalid')).toBe('true');
});

it('NAV-009: signing in with a changed address signs in to that hub, stores the address and loads the client there', async () => {
  const target = open();
  await reveal(target);
  type(target, 'hub-address', 'hub2.example.org');
  type(target, 'username', 'ann');
  type(target, 'password', 'pw');
  await submit(target, 0);
  await pasteInto(target, 0, '123456');
  expect(mocks.created.map((address) => address.origin)).toEqual(['https://hub2.example.org']);
  expect(hub.sent).toEqual([]);
  expect(mocks.writeAccessToken).toHaveBeenCalledWith(mocks.created[0], 'tok-moved');
  expect(localStorage.getItem('anima.hubAddress')).toBe('https://hub2.example.org');
  expect(mocks.loadClientAt).toHaveBeenCalledExactlyOnceWith('/devices');
});

it('NAV-009: signing in with the address left as it was uses the current hub and keeps the router move', async () => {
  const target = open();
  await reveal(target);
  type(target, 'username', 'ann');
  type(target, 'password', 'pw');
  await submit(target, 0);
  await pasteInto(target, 0, '123456');
  expect(mocks.created).toEqual([]);
  expect(hub.sent.map((sent) => sent.operation)).toEqual(['Login']);
  expect(mocks.goToDevices).toHaveBeenCalledOnce();
});

it('NAV-009: signing in with an invalid address sends nothing and keeps the typed credentials', async () => {
  const target = open();
  await reveal(target);
  type(target, 'hub-address', 'http://user:secretpw@hub.example.org');
  type(target, 'username', 'ann');
  type(target, 'password', 'pw');
  await submit(target, 0);
  expect(hub.sent).toEqual([]);
  expect(mocks.created).toEqual([]);
  expect(input(target, 'password').value).toBe('pw');
  expect(document.activeElement).toBe(input(target, 'hub-address'));
});

it('CLI-002: a browser shows no hub line', () => {
  mocks.desktop = false;
  const target = open();
  expect(target.textContent).not.toContain('Hub:');
  expect(target.querySelectorAll('button')).toHaveLength(1);
});
