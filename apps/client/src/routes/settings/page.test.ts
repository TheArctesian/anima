// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { failure, fakeHub, ok } from '$lib/access/access.fixture';
import { SETTINGS } from '$lib/nav/nav';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({
  hub: undefined as unknown,
  clearAccessToken: vi.fn(),
  goToSignIn: vi.fn(),
  platform: undefined as Capabilities | undefined,
}));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/desktop/token', () => ({ clearAccessToken: mocks.clearAccessToken }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => mocks.platform }));
vi.mock('$lib/access/navigation', () => ({ goToSignIn: mocks.goToSignIn }));
// The desktop rows need the shell's update store; stand-ins let the page's sections be counted (UPD-006).
vi.mock('$lib/updates/UpdatesRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('$lib/updates/tests/RowStub.svelte')).default }));
vi.mock('$lib/process-log/LogFolderRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('$lib/updates/tests/LogRowStub.svelte')).default }));
// The API keys section loads its own data and has its own tests in `$lib/api-keys/` (WP-50 F-3, F-10).
vi.mock('$lib/api-keys/ApiKeysSection.svelte', () => ({ default: () => undefined }));
// The Data retention section loads its own data and has its own tests in `$lib/retention/` (WP-50 F-3, F-10).
vi.mock('$lib/retention/RetentionSection.svelte', () => ({ default: () => undefined }));

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
  mocks.platform = BROWSER;
  mocks.clearAccessToken.mockReset().mockResolvedValue(undefined);
  mocks.goToSignIn.mockReset();
  for (const spy of consoleSpies) spy.mockClear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
});

it('SET-001: the nav link to Settings is /settings and the page renders the h1 "Settings"', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(SETTINGS.href).toBe('/settings');
  expect(target.querySelector('h1')?.textContent).toBe(SETTINGS.label);
});

/** Each section's heading, from the `aria-labelledby` that names it, so a section without its heading fails. */
function sectionHeadings(target: HTMLElement): string[] {
  return [...target.querySelectorAll(':scope > section')].map((section) => {
    const id = section.getAttribute('aria-labelledby') ?? '';
    const heading = section.querySelector(`:scope > h2[id="${id}"]`);
    return heading === section.firstElementChild ? (heading?.textContent ?? '') : '';
  });
}

function sectionTexts(target: HTMLElement, heading: string): string[] {
  const section = [...target.querySelectorAll(':scope > section')].find((each) => each.querySelector('h2')?.textContent === heading);
  if (section === undefined) throw new Error(`no ${heading} section`);
  return [...section.querySelectorAll('h2, p, button, span, option')].map((element) => element.textContent.trim());
}

it('SET-001, UI-005, MOT-043: the page is the title, then Account, then Appearance with Theme, Reduce motion and the two fonts, then Data retention, then API keys', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(target.firstElementChild?.tagName).toBe('H1');
  expect(sectionHeadings(target)).toEqual(['Account', 'Appearance', 'Data retention', 'API keys']);
  expect(sectionTexts(target, 'Account')).toEqual(['Account', 'Signed in as ann', 'Sign out']);
  const fonts = ['Fira Code', 'OpenDyslexic'];
  expect(sectionTexts(target, 'Appearance')).toEqual([
    'Appearance', 'Theme', 'Dark', 'Light', 'System', 'Reduce motion', 'System', 'On', 'Off',
    'Interface font', ...fonts, 'Monospace font', ...fonts,
  ]);
  expect(target.querySelectorAll('[role="switch"], img, input, textarea, a, [tabindex]:not(button)')).toHaveLength(0);
});

it('SET-001, UPD-006: the desktop app adds the Updates section last, holding the update and log folder rows', async () => {
  mocks.platform = DESKTOP_MAC;
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(sectionHeadings(target)).toEqual(['Account', 'Appearance', 'Data retention', 'API keys', 'Updates']);
  const updates = target.querySelector('section[aria-labelledby="settings-updates"]');
  expect(updates?.querySelectorAll('[data-stub]')).toHaveLength(2);
  expect(target.querySelectorAll('[data-stub]')).toHaveLength(2);
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
  const who = () => target.querySelector('section[aria-labelledby="settings-account"] p.who');
  const row = () => [...(who()?.parentElement?.children ?? [])].map((element) => element.tagName);
  const before = row();
  expect(before).toEqual(['P', 'BUTTON']);
  // The two lines are there before the name is: the label is laid out but hidden, and the name's line is empty.
  expect(who()?.classList.contains('pending')).toBe(true);
  expect(who()?.querySelector('strong')?.textContent).toBe('');
  release(ok({ me: { username: 'ann' } }));
  await settled();
  expect(row()).toEqual(before);
  expect(who()?.classList.contains('pending')).toBe(false);
});

it('NAV-005: the Sign out button is focusable by Tab in WebKit (tabindex 0)', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(signOutButton(target).getAttribute('tabindex')).toBe('0');
});
