// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({ join: vi.fn(), clear: vi.fn(), loadClientAt: vi.fn() }));

vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/desktop/link/shell', () => ({ joinLink: mocks.join }));
vi.mock('$lib/desktop/token', () => ({ clearAccessToken: mocks.clear }));
vi.mock('$lib/access/navigation', () => ({ loadClientAt: mocks.loadClientAt }));

const INVITE = 'anima-invite.1.c2VjcmV0LWludml0ZQ';
let app: ReturnType<typeof mount> | undefined;
const consoleSpies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

function open(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

async function paste(target: HTMLElement, text: string): Promise<void> {
  const field = target.querySelector('textarea');
  if (field === null) throw new Error('no field');
  field.value = text;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  target.querySelector('form')?.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  await settle();
  await settle();
}

beforeEach(() => {
  installAnimations();
  mocks.join.mockReset();
  mocks.clear.mockReset().mockResolvedValue(undefined);
  mocks.loadClientAt.mockReset();
  for (const spy of consoleSpies) spy.mockClear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
});

it('FRN-005: the invite field is monospace text with no autocomplete or spell check', () => {
  const field = open().querySelector('textarea');
  expect([field?.getAttribute('autocomplete'), field?.getAttribute('spellcheck'), field?.getAttribute('autocapitalize')]).toEqual(['off', 'false', 'off']);
  expect(document.querySelector('label[for="invite"]')?.textContent).toBe('Invite');
});

it('FRN-005, S3: Join clears the old token, saves the link address, shows the device key, then goes to sign-in', async () => {
  mocks.join.mockResolvedValue({ ok: true, value: { fingerprint: 'ab12-cd34', port: 41_000 } });
  const target = open();
  await paste(target, INVITE);
  expect(mocks.join).toHaveBeenCalledWith(INVITE);
  expect(mocks.clear.mock.calls[0]?.[0]).toMatchObject({ origin: 'http://127.0.0.1:41000' });
  expect(localStorage.getItem('anima.hubAddress')).toBe('http://127.0.0.1:41000');
  expect(target.querySelector('.fingerprint')?.textContent).toBe('ab12-cd34');
  expect(target.textContent).toContain("Compare it with the hub's device list.");
  [...target.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Sign in')?.click();
  expect(mocks.loadClientAt).toHaveBeenCalledWith('/sign-in');
});

it('LNK-003, LOG-007: a used invite says why, saves nothing and never shows or logs the invite', async () => {
  const message = 'The hub did not answer: the invite is used, expired or wrong, or the hub is off.';
  mocks.join.mockResolvedValue({ ok: false, problem: 'no-answer', message });
  const target = open();
  await paste(target, INVITE);
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(message);
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  expect(target.querySelector('textarea')?.value).toBe('');
  expect(document.body.innerHTML).not.toContain(INVITE);
  for (const spy of consoleSpies) expect(spy).not.toHaveBeenCalled();
});

it('FRN-005: a join refused before the hub saw the invite leaves it in the field', async () => {
  mocks.join.mockResolvedValue({ ok: false, problem: 'busy', message: 'Another link change is running. Wait for it.' });
  const target = open();
  await paste(target, INVITE);
  expect(target.querySelector('textarea')?.value).toBe(INVITE);
});

it('FRN-005, S3: joined but the old token stays: the key shows, Retry finishes without the invite, then sign-in', async () => {
  mocks.join.mockResolvedValue({ ok: true, value: { fingerprint: 'ab12-cd34', port: 41_000 } });
  mocks.clear.mockRejectedValueOnce(new Error('denied'));
  const target = open();
  await paste(target, INVITE);
  expect(target.querySelector('.fingerprint')?.textContent).toBe('ab12-cd34');
  expect(target.querySelector('[role="alert"]')?.textContent).toContain('Retry.');
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  const buttons = () => [...target.querySelectorAll('button')].map((button) => button.textContent.trim());
  expect(buttons()).toEqual(['Retry']);
  [...target.querySelectorAll('button')][0]?.click();
  await settle();
  expect(mocks.join).toHaveBeenCalledOnce();
  expect(localStorage.getItem('anima.hubAddress')).toBe('http://127.0.0.1:41000');
  expect(buttons()).toEqual(['Sign in']);
});
