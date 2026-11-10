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
vi.mock('$lib/access/navigation', () => ({ goToSignIn: mocks.goToSignIn, loadClientAt: vi.fn() }));
// The desktop rows need the shell's update store; stand-ins let the page's sections be counted (UPD-006).
vi.mock('$lib/updates/UpdatesRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('$lib/updates/tests/RowStub.svelte')).default }));
vi.mock('$lib/process-log/LogFolderRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('$lib/updates/tests/LogRowStub.svelte')).default }));
// The API keys section loads its own data and has its own tests in `$lib/api-keys/` (WP-50 F-3, F-10).
vi.mock('$lib/api-keys/ApiKeysSection.svelte', () => ({ default: () => undefined }));
// The default check settings section loads its own data and has its own tests in `$lib/checks/defaults/` (WP-50 F-3, F-10).
vi.mock('$lib/checks/defaults/DefaultsSection.svelte', () => ({ default: () => undefined }));
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

/** The page's sections, each named by its heading through `aria-labelledby`. */
function sections(target: HTMLElement): HTMLElement[] {
  return [...target.querySelectorAll<HTMLElement>('section[aria-labelledby]')];
}

/** Each section's name, from the heading its `aria-labelledby` points at, so a section without its heading fails. */
function sectionHeadings(target: HTMLElement): string[] {
  return sections(target).map((section) => section.querySelector(`h2[id="${section.getAttribute('aria-labelledby') ?? ''}"]`)?.textContent ?? '');
}

/** The text a person reads in an element: decorative icons (aria-hidden) are left out. */
function readable(element: Element): string {
  const copy = element.cloneNode(true) as Element;
  for (const hidden of copy.querySelectorAll('[aria-hidden="true"]')) hidden.remove();
  return copy.textContent.trim();
}

function sectionTexts(target: HTMLElement, heading: string): string[] {
  const section = sections(target).find((each) => each.querySelector('h2')?.textContent === heading);
  if (section === undefined) throw new Error(`no ${heading} section`);
  return [...section.querySelectorAll('h2, p, label, button, option')].map(readable);
}

it('SET-001, UI-005, MOT-043: the page is the title, then Account, then Appearance with Theme, Reduce motion and the two fonts, then Default check settings, then Data retention, then API keys', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(target.querySelector('h1')?.compareDocumentPosition(sections(target)[0] as Node)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  expect(sectionHeadings(target)).toEqual(['Account', 'Appearance', 'Default check settings', 'Data retention', 'API keys']);
  expect(sectionTexts(target, 'Account')).toEqual(['Account', 'Who is signed in to this hub.', 'Signed in as ann', 'Sign out', '']);
  const fonts = ['Fira Code', 'OpenDyslexic'];
  expect(sectionTexts(target, 'Appearance')).toEqual([
    'Appearance', 'How Anima looks on this device.', 'Theme', 'Dark', 'Light', 'System', 'Reduce motion', 'System', 'On', 'Off',
    'Interface font', ...fonts, 'Monospace font', ...fonts,
  ]);
  expect(target.querySelectorAll('[role="switch"], img, input, textarea, a, [tabindex]:not(button)')).toHaveLength(0);
});

it('SET-001, UPD-006: the desktop app adds the Updates section before API keys, holding the update and log folder rows', async () => {
  mocks.platform = DESKTOP_MAC;
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(sectionHeadings(target)).toEqual(['Account', 'Appearance', 'Default check settings', 'Data retention', 'Updates', 'API keys']);
  const updates = target.querySelector('section[aria-labelledby="settings-updates"]');
  expect(updates?.querySelectorAll('[data-stub]')).toHaveLength(2);
  expect(target.querySelectorAll('[data-stub]')).toHaveLength(2);
});

it('CLI-003: Sign out calls logout, clears the token with hub.address, then goes to sign-in', async () => {
  mocks.platform = DESKTOP_MAC;
  const target = await open({ Me: ok({ me: { username: 'ann' } }), Logout: ok({ logout: true }) });
  signOutButton(target).click();
  await settled();
  expect(hub.sent.map((request) => request.operation)).toEqual(['Me', 'Logout']);
  expect(mocks.clearAccessToken).toHaveBeenCalledWith(hub.hub.address);
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
});

it('CLI-003: Sign out clears the token and goes to sign-in even when logout fails', async () => {
  mocks.platform = DESKTOP_MAC;
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
  expect(readable(signOutButton(target))).toBe('Sign out');
});

it('LOG-007: nothing is written to the console while signing out', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }), Logout: ok({ logout: true }) });
  signOutButton(target).click();
  await settled();
  for (const spy of consoleSpies) expect(spy).not.toHaveBeenCalled();
});

it('SET-002: Sign out disables the button while the hub answers, so a second click sends no second logout', async () => {
  mocks.platform = DESKTOP_MAC;
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

it('MOT-007: the name, the Sign out button and the alert line are laid out before the name arrives, and stay', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const pending = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ Me: pending });
  const account = () => target.querySelector('section[aria-labelledby="settings-account"]');
  const who = () => account()?.querySelector('p.who');
  const before = [who(), signOutButton(target), account()?.querySelector('[role="alert"]')];
  expect(before.every((element) => element !== null && element !== undefined)).toBe(true);
  // The line is there before the name is: laid out but hidden, with an empty name.
  expect(who()?.classList.contains('pending')).toBe(true);
  expect(who()?.querySelector('strong')?.textContent).toBe('');
  release(ok({ me: { username: 'ann' } }));
  await settled();
  const after = [who(), signOutButton(target), account()?.querySelector('[role="alert"]')];
  expect(after.every((element, index) => element === before[index])).toBe(true);
  expect(who()?.classList.contains('pending')).toBe(false);
});

it('NAV-005: the Sign out button is focusable by Tab in WebKit (tabindex 0)', async () => {
  const target = await open({ Me: ok({ me: { username: 'ann' } }) });
  expect(signOutButton(target).getAttribute('tabindex')).toBe('0');
});
