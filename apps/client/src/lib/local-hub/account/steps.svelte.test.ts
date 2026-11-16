// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import AccountStep from './AccountStep.svelte';
import AuthenticatorStep from './AuthenticatorStep.svelte';

const mocks = vi.hoisted(() => ({ newTotp: vi.fn(), create: vi.fn() }));
vi.mock('$lib/desktop/local-hub/shell', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/local-hub/shell')>()),
  newTotp: mocks.newTotp,
  createLocalAccount: mocks.create,
}));

const SECRET = 'JBSWY3DPEHPK3PXP';
const URI = `otpauth://totp/Anima:ann?secret=${SECRET}&issuer=Anima`;
const PASSWORD = 'correct horse battery';
let app: ReturnType<typeof mount> | undefined;
const consoleSpies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

function show<P extends Record<string, unknown>>(component: Parameters<typeof mount>[0], props: P): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(component, { target, props });
  flushSync();
  return target;
}

function typeInto(target: HTMLElement, index: number, value: string): void {
  const field = target.querySelectorAll('input')[index];
  if (field === undefined) throw new Error('no field');
  field.value = value;
  field.dispatchEvent(new Event('input', { bubbles: true }));
}

/** A code pasted into the first box. */
async function enterCode(target: HTMLElement, code: string): Promise<void> {
  const event = new Event('paste', { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'clipboardData', { value: { getData: () => code } });
  target.querySelector('[role="group"] input')?.dispatchEvent(event);
  await settle();
  await settle();
}

beforeEach(() => {
  mocks.newTotp.mockReset().mockResolvedValue({ ok: true, value: { secret: SECRET, uri: URI } });
  mocks.create.mockReset().mockResolvedValue({ ok: true, value: null });
  for (const spy of consoleSpies) spy.mockClear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('FRN-003: the account step refuses a bad name and a short password, then hands on the name and password', async () => {
  const onnext = vi.fn();
  const target = show(AccountStep, { heading: 'Make your account', onnext });
  expect(document.activeElement).toBe(target.querySelector('h2'));
  typeInto(target, 0, '.ann');
  typeInto(target, 1, 'short');
  target.querySelector('form')?.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  await settle();
  expect(onnext).not.toHaveBeenCalled();
  expect(target.textContent).toContain('starting with a letter or a digit');
  expect(target.textContent).toContain('Use at least 12 characters.');
  typeInto(target, 0, 'ann');
  typeInto(target, 1, PASSWORD);
  typeInto(target, 2, PASSWORD);
  target.querySelector('form')?.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  await settle();
  expect(onnext).toHaveBeenCalledWith({ name: 'ann', password: PASSWORD });
  expect(target.querySelectorAll('input')[1]?.value).toBe('');
});

it('FRN-004, LOG-007: the authenticator step draws the URI as a QR code labelled without the secret, and shows the key grouped', async () => {
  const target = show(AuthenticatorStep, { heading: 'Add Anima.ing', name: 'ann', password: PASSWORD, onmade: vi.fn(), onback: vi.fn() });
  await settle();
  expect(mocks.newTotp).toHaveBeenCalledWith('ann');
  const qr = target.querySelector('svg[role="img"]');
  expect(qr?.getAttribute('aria-label')).toBe('Authenticator code for Anima');
  expect(target.querySelector('output')?.textContent).toBe('JBSW Y3DP EHPK 3PXP');
  for (const element of target.querySelectorAll('*')) {
    for (const attribute of element.attributes) expect(attribute.value).not.toContain(SECRET);
  }
});

it('FRN-004: a wrong code says to wait for a new one; a right code makes the account and drops the secret', async () => {
  const onmade = vi.fn();
  mocks.create.mockResolvedValueOnce({ ok: false, problem: 'refused', message: 'x' });
  const target = show(AuthenticatorStep, { heading: 'Add Anima.ing', name: 'ann', password: PASSWORD, onmade, onback: vi.fn() });
  await settle();
  await enterCode(target, '111111');
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('The code did not match. Wait for a new code and try again.');
  await enterCode(target, '222222');
  expect(mocks.create).toHaveBeenLastCalledWith({ name: 'ann', password: PASSWORD, secret: SECRET, code: '222222' });
  expect(onmade).toHaveBeenCalledOnce();
  expect(target.textContent).not.toContain('JBSW');
  for (const spy of consoleSpies) expect(spy).not.toHaveBeenCalled();
});
