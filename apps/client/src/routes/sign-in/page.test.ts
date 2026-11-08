// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { failure, fakeHub, ok } from '$lib/access/access.fixture';
import { TOKEN_UNREADABLE } from '$lib/hub/error';
import type { Hub } from '$lib/hub/hub';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({
  hub: undefined as unknown,
  writeAccessToken: vi.fn(),
  goToDevices: vi.fn(),
  reloadClient: vi.fn(),
  desktop: true,
}));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/desktop/token', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/token')>()),
  writeAccessToken: mocks.writeAccessToken,
}));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER, DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => (mocks.desktop ? DESKTOP_MAC : BROWSER) };
});
vi.mock('$lib/access/navigation', () => ({ goToDevices: mocks.goToDevices, reloadClient: mocks.reloadClient }));

const TOKEN = 'tok-secret-9f2';
const PASSWORD = 'hunter2-pw';
let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;
const consoleSpies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));

function open(answers: Parameters<typeof fakeHub>[0]): HTMLElement {
  hub = fakeHub(answers);
  mocks.hub = hub.hub satisfies Hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

function input(target: HTMLElement, id: string): HTMLInputElement {
  const element = target.querySelector<HTMLInputElement>(`#${id}`);
  if (element === null) throw new Error(`no #${id}`);
  return element;
}

function type(target: HTMLElement, id: string, text: string): void {
  const element = input(target, id);
  element.value = text;
  element.dispatchEvent(new Event('input', { bubbles: true }));
}

async function submit(target: HTMLElement, formIndex: number): Promise<void> {
  target.querySelectorAll('form')[formIndex]?.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => {
    flushSync();
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

async function signInWith(target: HTMLElement, code = '123456'): Promise<void> {
  type(target, 'username', 'ann');
  type(target, 'password', PASSWORD);
  type(target, 'totp', code);
  await submit(target, 0);
}

beforeEach(() => {
  mocks.writeAccessToken.mockReset().mockResolvedValue(undefined);
  mocks.goToDevices.mockReset();
  mocks.reloadClient.mockReset();
  mocks.desktop = true;
  for (const spy of consoleSpies) spy.mockClear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
});

it('DEV-011, OD-2: the fields are labelled; the password is hidden; the code is six numeric digits', () => {
  const target = open({});
  for (const id of ['username', 'password', 'totp', 'hub-address']) {
    expect(target.querySelector(`label[for="${id}"]`)).not.toBeNull();
  }
  expect(input(target, 'password').type).toBe('password');
  const code = input(target, 'totp');
  expect(code.getAttribute('inputmode')).toBe('numeric');
  expect(code.getAttribute('autocomplete')).toBe('one-time-code');
  expect(code.maxLength).toBe(6);
});

it('CLI-003: a good sign-in sends totpCode, writes the token with hub.address and goes to /devices', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await signInWith(target);
  expect(hub.sent).toEqual([{ operation: 'Login', variables: { input: { username: 'ann', password: PASSWORD, totpCode: '123456' } } }]);
  expect(mocks.writeAccessToken).toHaveBeenCalledWith(hub.hub.address, TOKEN);
  expect(hub.hub.address.origin).not.toBe('https://hub.anima.ing');
  expect(mocks.goToDevices).toHaveBeenCalledOnce();
});

it('LOG-007: after a submit the password and code fields are empty and nothing secret is in the DOM or the console', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await signInWith(target);
  expect(input(target, 'password').value).toBe('');
  expect(input(target, 'totp').value).toBe('');
  expect(document.body.innerHTML).not.toContain(PASSWORD);
  expect(document.body.innerHTML).not.toContain(TOKEN);
  for (const spy of consoleSpies) expect(JSON.stringify(spy.mock.calls)).not.toMatch(new RegExp(`${PASSWORD}|${TOKEN}`));
});

it('LOG-007: a wrong password shows a fixed message, not the hub text, and clears the password', async () => {
  const target = open({ Login: failure('UNAUTHENTICATED', `echo ${PASSWORD}`) });
  await signInWith(target);
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('The user name, password or code is wrong.');
  expect(document.body.innerHTML).not.toContain(PASSWORD);
  expect(input(target, 'password').value).toBe('');
  expect(mocks.writeAccessToken).not.toHaveBeenCalled();
  expect(mocks.goToDevices).not.toHaveBeenCalled();
});

it('NET-004: an unreachable hub shows the VPN message', async () => {
  const target = open({ Login: failure('unreachable') });
  await signInWith(target);
  expect(target.querySelector('[role="alert"]')?.textContent).toContain('Check the VPN client');
});

it('DEV-011: hub field errors show beside their fields', async () => {
  const error = { code: 'VALIDATION' as const, message: 'm', fields: [{ field: 'totpCode', message: 'The code is not accepted.' }] };
  const target = open({ Login: { ok: false, error } });
  await signInWith(target);
  expect(target.querySelector('#totp-problem')?.textContent).toBe('The code is not accepted.');
  expect(input(target, 'totp').getAttribute('aria-describedby')).toBe('totp-problem');
  expect(target.querySelector('#username-problem')?.textContent).toBe('');
});

it('OD-2: a code that is not six digits is refused before anything is sent', async () => {
  const target = open({});
  await signInWith(target, '12ab');
  expect(hub.sent).toEqual([]);
  expect(target.querySelector('#totp-problem')?.textContent).toBe('The code is six digits.');
});

it('CLI-002: a valid hub address is stored and the client reloads', async () => {
  const target = open({});
  type(target, 'hub-address', 'hub.example.org');
  await submit(target, 1);
  expect(localStorage.getItem('anima.hubAddress')).toBe('https://hub.example.org');
  expect(mocks.reloadClient).toHaveBeenCalledOnce();
});

it('NET-005: the address field starts at the current hub origin', () => {
  const target = open({});
  expect(input(target, 'hub-address').value).toBe(hub.hub.address.origin);
});

it('CLI-002: an invalid hub address stores nothing, does not reload and shows the error without the input', async () => {
  const target = open({});
  type(target, 'hub-address', 'http://user:secretpw@hub.example.org');
  await submit(target, 1);
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  expect(mocks.reloadClient).not.toHaveBeenCalled();
  const problem = target.querySelector('#address-problem')?.textContent ?? '';
  expect(problem).not.toBe('');
  expect(problem).not.toContain('secretpw');
});

it('OD-2: the user name is sent without surrounding spaces', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  type(target, 'username', ' ann ');
  type(target, 'password', PASSWORD);
  type(target, 'totp', '123456');
  await submit(target, 0);
  expect(hub.sent[0]?.variables).toEqual({ input: { username: 'ann', password: PASSWORD, totpCode: '123456' } });
});

it('CLI-003: an unreadable key store is not reported as a wrong password', async () => {
  const target = open({ Login: { ok: false, error: TOKEN_UNREADABLE } });
  await signInWith(target);
  const text = target.querySelector('[role="alert"]')?.textContent ?? '';
  expect(text).toContain('key store');
  expect(text).not.toContain('wrong');
});

it('CLI-003: outside the desktop app nothing is sent to the hub and a fixed message shows', async () => {
  mocks.desktop = false;
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await signInWith(target);
  expect(hub.sent).toEqual([]);
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('Signing in needs the Anima desktop app, which keeps the access token.');
});

it('DEV-011: after a failed submit the invalid fields are aria-invalid and the first one has focus', async () => {
  const target = open({});
  type(target, 'username', 'ann');
  type(target, 'password', '');
  type(target, 'totp', '12ab');
  await submit(target, 0);
  expect(input(target, 'password').getAttribute('aria-invalid')).toBe('true');
  expect(input(target, 'totp').getAttribute('aria-invalid')).toBe('true');
  expect(input(target, 'username').hasAttribute('aria-invalid')).toBe(false);
  expect(document.activeElement).toBe(input(target, 'password'));
});

it('DEV-011: a field stops being aria-invalid once its value passes the rules', async () => {
  const target = open({});
  type(target, 'username', 'ann');
  type(target, 'password', PASSWORD);
  type(target, 'totp', '12ab');
  await submit(target, 0);
  expect(input(target, 'totp').getAttribute('aria-invalid')).toBe('true');
  type(target, 'totp', '123456');
  flushSync();
  expect(input(target, 'totp').hasAttribute('aria-invalid')).toBe(false);
  expect(target.querySelector('#totp-problem')?.textContent).toBe('');
});

it('DEV-011: hub field errors also mark the field and take focus', async () => {
  const error = { code: 'VALIDATION' as const, message: 'm', fields: [{ field: 'totpCode', message: 'The code is not accepted.' }] };
  const target = open({ Login: { ok: false, error } });
  await signInWith(target);
  expect(input(target, 'totp').getAttribute('aria-invalid')).toBe('true');
  expect(document.activeElement).toBe(input(target, 'totp'));
});

it('NAV-005: the Sign in and Save address buttons are focusable by Tab in WebKit (tabindex 0)', () => {
  const target = open({});
  const buttons = [...target.querySelectorAll('button')];
  expect(buttons).toHaveLength(2);
  for (const button of buttons) expect(button.getAttribute('tabindex')).toBe('0');
});
