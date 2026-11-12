// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { failure, fakeHub, ok } from '$lib/access/access.fixture';
import { TOKEN_UNREADABLE, type Hub } from '$lib/hub/hub';
import Page from './+page.svelte';
import { boxes, continueWith, input, pasteInto, press, problemOf, settle, submit, type, typeCode } from './page.fixture';

const mocks = vi.hoisted(() => ({
  hub: undefined as unknown,
  writeAccessToken: vi.fn(),
  goToDevices: vi.fn(),
  reloadClient: vi.fn(),
  loadClientAt: vi.fn(),
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
vi.mock('$lib/access/navigation', () => ({ goToDevices: mocks.goToDevices, reloadClient: mocks.reloadClient, loadClientAt: mocks.loadClientAt }));

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

/** Both steps: the user name and password, then the code pasted into the first box ('' skips the code). */
async function signInWith(target: HTMLElement, code = '123456'): Promise<void> {
  await continueWith(target, 'ann', PASSWORD);
  if (code === '') {
    clickLink(target, 'Sign in without a code');
    await settle();
  } else {
    await pasteInto(target, 0, code);
  }
}

function clickLink(target: HTMLElement, name: string): void {
  const link = [...target.querySelectorAll('button')].find((button) => button.textContent.trim() === name);
  if (link === undefined) throw new Error(`no ${name}`);
  link.click();
  flushSync();
}

beforeEach(() => {
  mocks.writeAccessToken.mockReset().mockResolvedValue(undefined);
  mocks.goToDevices.mockReset();
  mocks.reloadClient.mockReset();
  mocks.loadClientAt.mockReset();
  mocks.desktop = true;
  for (const spy of consoleSpies) spy.mockClear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
  sessionStorage.clear();
});

it('DEV-011: step 1 has a labelled user name and a hidden password, and one Continue button; there is no code field yet', () => {
  const target = open({});
  for (const id of ['username', 'password']) expect(target.querySelector(`label[for="${id}"]`)).not.toBeNull();
  expect(input(target, 'password').type).toBe('password');
  expect(boxes(target)).toHaveLength(0);
  expect(target.querySelector('button[type="submit"]')?.textContent.trim()).toBe('Continue');
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
  expect(boxes(target).map((box) => box.value).join('')).toBe('');
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

it('NAV-009: Continue with an empty password or user name sends nothing, marks the field and focuses it', async () => {
  const target = open({});
  await continueWith(target, 'ann', '');
  expect(hub.sent).toEqual([]);
  expect(boxes(target)).toHaveLength(0);
  expect(problemOf(target, 'password')).toBe('Type your password.');
  expect(input(target, 'password').getAttribute('aria-invalid')).toBe('true');
  expect(document.activeElement).toBe(input(target, 'password'));
  await continueWith(target, ' ', PASSWORD);
  expect(document.activeElement).toBe(input(target, 'username'));
});

it('NAV-009: a field stops being aria-invalid once its value passes the rules', async () => {
  const target = open({});
  await continueWith(target, 'ann', '');
  type(target, 'password', PASSWORD);
  flushSync();
  expect(input(target, 'password').hasAttribute('aria-invalid')).toBe(false);
  expect(problemOf(target, 'password')).toBe('');
});

it('NAV-009: Continue sends nothing and shows step 2: a named group of six boxes, the first focused', async () => {
  const target = open({});
  await continueWith(target, 'ann', PASSWORD);
  expect(hub.sent).toEqual([]);
  expect(target.querySelector('h2')?.textContent).toBe('Enter the code from your authenticator app');
  expect(target.querySelector('[role="group"]')?.getAttribute('aria-label')).toBe('Authenticator code');
  const six = boxes(target);
  expect(six.map((box) => box.getAttribute('aria-label'))).toEqual([1, 2, 3, 4, 5, 6].map((n) => `Digit ${n.toString()} of 6`));
  for (const box of six) expect(box.getAttribute('inputmode')).toBe('numeric');
  expect(six[0]?.getAttribute('autocomplete')).toBe('one-time-code');
  expect(document.activeElement).toBe(six[0]);
});

it('NAV-009: typing six digits moves along the boxes and submits once, on the sixth', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await continueWith(target, 'ann', PASSWORD);
  press(target, 0, '1');
  expect(document.activeElement).toBe(boxes(target)[1]);
  expect(press(target, 1, 'x').defaultPrevented).toBe(true);
  expect(boxes(target)[1]?.value).toBe('');
  await typeCode(target, '123456');
  press(target, 5, '7');
  await settle();
  expect(hub.sent).toEqual([{ operation: 'Login', variables: { input: { username: 'ann', password: PASSWORD, totpCode: '123456' } } }]);
});

it('NAV-009: Backspace on an empty box clears the previous one; the arrows move without changing digits', async () => {
  const target = open({});
  await continueWith(target, 'ann', PASSWORD);
  press(target, 0, '1');
  press(target, 1, '2');
  press(target, 2, 'ArrowLeft');
  expect(document.activeElement).toBe(boxes(target)[1]);
  press(target, 1, 'ArrowRight');
  expect(document.activeElement).toBe(boxes(target)[2]);
  press(target, 2, 'Backspace');
  expect(boxes(target).map((box) => box.value).join('')).toBe('1');
  expect(document.activeElement).toBe(boxes(target)[1]);
});

it('NAV-009: pasting six digits into any box fills all six and submits; a partial paste does not submit', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await continueWith(target, 'ann', PASSWORD);
  await pasteInto(target, 2, '12 34');
  expect(hub.sent).toEqual([]);
  await pasteInto(target, 3, 'code 654321');
  expect(hub.sent).toHaveLength(1);
  expect(hub.sent[0]?.variables).toEqual({ input: { username: 'ann', password: PASSWORD, totpCode: '654321' } });
});

it('NAV-009: Back returns to step 1 with the user name and password kept', async () => {
  const target = open({});
  await continueWith(target, 'ann', PASSWORD);
  clickLink(target, 'Back');
  await settle();
  expect(input(target, 'username').value).toBe('ann');
  expect(input(target, 'password').value).toBe(PASSWORD);
  expect(hub.sent).toEqual([]);
});

it('NAV-009, OD-2: Sign in without a code sends no totpCode', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await signInWith(target, '');
  expect(hub.sent).toEqual([{ operation: 'Login', variables: { input: { username: 'ann', password: PASSWORD } } }]);
});

it('NAV-009, SEC-002: a refusal returns to step 1 with the message, the user name kept, the password cleared and focused', async () => {
  const target = open({ Login: failure('UNAUTHENTICATED') });
  await signInWith(target);
  expect(boxes(target)).toHaveLength(0);
  expect(input(target, 'username').value).toBe('ann');
  expect(input(target, 'password').value).toBe('');
  expect(document.activeElement).toBe(input(target, 'password'));
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('The user name, password or code is wrong.');
  await continueWith(target, 'ann', PASSWORD);
  expect(boxes(target).map((box) => box.value).join('')).toBe('');
});

it('DEV-011: a hub field error shows beside its field on step 1 and takes focus', async () => {
  const error = { code: 'VALIDATION' as const, message: 'm', fields: [{ field: 'username', message: 'No such name.' }] };
  const target = open({ Login: { ok: false, error } });
  await signInWith(target);
  expect(problemOf(target, 'username')).toBe('No such name.');
  expect(input(target, 'username').getAttribute('aria-describedby')).toBe('username-problem');
  expect(document.activeElement).toBe(input(target, 'username'));
});

it('CLI-003: an unreadable key store is not reported as a wrong password', async () => {
  const target = open({ Login: { ok: false, error: TOKEN_UNREADABLE } });
  await signInWith(target);
  const text = target.querySelector('[role="alert"]')?.textContent ?? '';
  expect(text).toContain('key store');
  expect(text).not.toContain('wrong');
});

it('OD-2: the user name is sent without surrounding spaces', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await continueWith(target, ' ann ', PASSWORD);
  await pasteInto(target, 0, '123456');
  expect(hub.sent[0]?.variables).toEqual({ input: { username: 'ann', password: PASSWORD, totpCode: '123456' } });
});

it('NAV-005: the Continue and Save address buttons are focusable by Tab in WebKit (tabindex 0)', () => {
  const target = open({});
  const buttons = [...target.querySelectorAll('button')];
  expect(buttons).toHaveLength(2);
  for (const button of buttons) expect(button.getAttribute('tabindex')).toBe('0');
});

it('CLI-002: the desktop app never sends SignInBrowser and keeps the page on a router move', async () => {
  const target = open({ Login: ok({ login: { token: TOKEN } }) });
  await signInWith(target);
  expect(hub.sent.map((sent) => sent.operation)).toEqual(['Login']);
  expect(mocks.loadClientAt).not.toHaveBeenCalled();
});

describe('in a browser', () => {
  const SIGNED_IN = ok({ signInBrowser: { expiresAt: '2026-11-14T10:00:00Z' } });

  beforeEach(() => {
    mocks.desktop = false;
  });

  it('D-3, E-4: a sign-in sends SignInBrowser with the form input, never Login, and loads /devices as a new page', async () => {
    const target = open({ SignInBrowser: SIGNED_IN, Login: ok({ login: { token: TOKEN } }) });
    await signInWith(target);
    expect(hub.sent).toEqual([{ operation: 'SignInBrowser', variables: { input: { username: 'ann', password: PASSWORD, totpCode: '123456' } } }]);
    expect(mocks.loadClientAt).toHaveBeenCalledExactlyOnceWith('/devices');
    expect(mocks.goToDevices).not.toHaveBeenCalled();
    expect(mocks.writeAccessToken).not.toHaveBeenCalled();
  });

  it('OD-2: an empty code is left out of the input, as on desktop', async () => {
    const target = open({ SignInBrowser: SIGNED_IN });
    await signInWith(target, '');
    expect(hub.sent[0]?.variables).toEqual({ input: { username: 'ann', password: PASSWORD } });
  });

  it('CLI-003, LOG-007: after a good sign-in nothing secret is in the DOM, the console or web storage', async () => {
    const target = open({ SignInBrowser: SIGNED_IN });
    await signInWith(target);
    expect(boxes(target).map((box) => box.value).join('')).toBe('');
    const kept = [document.body.innerHTML, JSON.stringify({ ...localStorage }), JSON.stringify({ ...sessionStorage }), document.cookie];
    for (const spy of consoleSpies) kept.push(JSON.stringify(spy.mock.calls));
    for (const text of kept) expect(text).not.toMatch(new RegExp(`${PASSWORD}|123456`));
  });

  it('D-3: the boxes and links stay off while the new page loads, so a second submit sends nothing', async () => {
    const target = open({ SignInBrowser: SIGNED_IN });
    await signInWith(target);
    expect(boxes(target).every((box) => box.disabled)).toBe(true);
    expect(target.querySelector('.links button:disabled')).not.toBeNull();
    await pasteInto(target, 0, '654321');
    expect(hub.sent).toHaveLength(1);
  });

  it('CLI-002: there is no hub address form, so the page shows one form and one button', () => {
    const target = open({});
    expect(target.querySelector('#hub-address')).toBeNull();
    expect(target.textContent).not.toContain('Hub address');
    expect(target.querySelectorAll('form')).toHaveLength(1);
    expect(target.querySelectorAll('button')).toHaveLength(1);
  });
});

describe('in a browser, a failed sign-in', () => {
  beforeEach(() => {
    mocks.desktop = false;
  });

  it.each([
    ['UNAUTHENTICATED', 'The user name, password or code is wrong.'],
    ['FORBIDDEN', 'The hub refused sign-in from here. Check the VPN client and the page address.'],
    ['malformed', 'The hub refused the request. Check the VPN client.'],
    ['unreachable', 'The hub did not answer. Check the VPN client.'],
  ] as const)('LOG-007: %s shows its fixed browser message, never the hub text, and loads no page', async (code, message) => {
    const target = open({ SignInBrowser: failure(code, `echo ${PASSWORD}`) });
    await signInWith(target);
    const shown = target.querySelector('[role="alert"]')?.textContent ?? '';
    expect(shown).toBe(message);
    expect(shown).not.toContain('hub address');
    expect(document.body.innerHTML).not.toContain(PASSWORD);
    expect(mocks.loadClientAt).not.toHaveBeenCalled();
    expect(target.querySelector('button')?.disabled).toBe(false);
  });

  it('NAV-009: after a refusal with no marked field the keyboard focus is on the password', async () => {
    const target = open({ SignInBrowser: failure('UNAUTHENTICATED') });
    await signInWith(target);
    expect(document.activeElement).toBe(input(target, 'password'));
  });

  it('DEV-011: hub field errors show beside their fields, as on desktop', async () => {
    const error = { code: 'VALIDATION' as const, message: 'm', fields: [{ field: 'username', message: 'No such name.' }] };
    const target = open({ SignInBrowser: { ok: false, error } });
    await signInWith(target);
    expect(problemOf(target, 'username')).toBe('No such name.');
    expect(document.activeElement).toBe(input(target, 'username'));
  });
});
