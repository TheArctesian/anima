// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { failure, fakeHub, ok } from '$lib/access/access.fixture';
import { SETTINGS } from '$lib/nav/nav';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown, clearAccessToken: vi.fn(), goToSignIn: vi.fn() }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/desktop/token', () => ({ clearAccessToken: mocks.clearAccessToken, keyStoreAvailable: () => false }));
vi.mock('$lib/access/navigation', () => ({ goToSignIn: mocks.goToSignIn }));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;
const consoleSpies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));

async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

async function open(answers: Parameters<typeof fakeHub>[0]): Promise<HTMLElement> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  await settled();
  return target;
}

function signOutButton(target: HTMLElement): HTMLButtonElement {
  const button = target.querySelector('button');
  if (button === null) throw new Error('no button');
  return button;
}

beforeEach(() => {
  mocks.clearAccessToken.mockReset().mockResolvedValue(undefined);
  mocks.goToSignIn.mockReset();
  for (const spy of consoleSpies) spy.mockClear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('SET-001: the nav link to Settings is /settings and the page renders the h1 "Settings"', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(SETTINGS.href).toBe('/settings');
  expect(target.querySelector('h1')?.textContent).toBe(SETTINGS.label);
});

it('SET-002: the page holds the title, "Signed in as <name>", "Sign out" and the two font choices, and nothing else', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  const texts = [...target.querySelectorAll('h1, p, button, span, option')].map((element) => element.textContent.trim());
  const fonts = ['Fira Code', 'OpenDyslexic'];
  expect(texts).toEqual(['Settings', 'Signed in as ann', 'Sign out', 'Interface font', ...fonts, 'Monospace font', ...fonts]);
  const elements = [...target.querySelectorAll('*')].map((element) => element.tagName.toLowerCase());
  const fontRow = ['label', 'span', 'select', 'option', 'option'];
  expect(elements).toEqual(['h1', 'section', 'p', 'strong', 'button', 'section', ...fontRow, ...fontRow]);
  expect(target.querySelectorAll('[role="switch"], img, input, textarea, a, [tabindex]:not(button)')).toHaveLength(0);
  expect(signOutButton(target).textContent).toBe('Sign out');
});

it('CLI-003: Sign out calls logout, clears the token with hub.address, then goes to sign-in', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }), Logout: ok({ logout: true }) });
  signOutButton(target).click();
  await settled();
  expect(hub.sent.map((request) => request.operation)).toEqual(['Me', 'Logout']);
  expect(mocks.clearAccessToken).toHaveBeenCalledWith(hub.hub.address);
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
});

it('CLI-003: Sign out clears the token and goes to sign-in even when logout fails', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }), Logout: failure('unreachable') });
  signOutButton(target).click();
  await settled();
  expect(mocks.clearAccessToken).toHaveBeenCalledWith(hub.hub.address);
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
});

it('API-003: an UNAUTHENTICATED me result goes to sign-in', async () => {
  await open({ Me: failure('UNAUTHENTICATED') });
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
});

it('NET-004: an unreachable hub does not go to sign-in, shows a fixed message and keeps Sign out', async () => {
  const target = await open({ Me: failure('unreachable', 'secret hub text') });
  expect(mocks.goToSignIn).not.toHaveBeenCalled();
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('Anima could not read who is signed in.');
  expect(signOutButton(target).textContent).toBe('Sign out');
});

it('LOG-007: nothing is written to the console while signing out', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }), Logout: ok({ logout: true }) });
  signOutButton(target).click();
  await settled();
  for (const spy of consoleSpies) expect(spy).not.toHaveBeenCalled();
});

it('SET-002: Sign out disables the button while the hub answers, so a second click sends no second logout', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const pending = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ Me: ok({ me: { username: 'ann' } }), Logout: pending });
  const button = signOutButton(target);
  button.click();
  flushSync();
  expect(button.disabled).toBe(true);
  button.click();
  await settled();
  expect(hub.sent.filter((request) => request.operation === 'Logout')).toHaveLength(1);
  release(ok({ logout: true }));
  await settled();
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
});

it('MOT-007: the Sign out button has the same place before and after the name arrives', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const pending = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ Me: pending });
  const before = [...target.querySelectorAll('section[aria-label="Account"] > *')].map((element) => element.tagName);
  expect(before).toEqual(['P', 'BUTTON']);
  expect(target.querySelector('section[aria-label="Account"] > p')?.textContent.trim()).toBe('');
  release(ok({ me: { username: 'ann' } }));
  await settled();
  const after = [...target.querySelectorAll('section[aria-label="Account"] > *')].map((element) => element.tagName);
  expect(after).toEqual(before);
});

it('NAV-005: the Sign out button is focusable by Tab in WebKit (tabindex 0)', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(signOutButton(target).getAttribute('tabindex')).toBe('0');
});
