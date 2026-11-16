// @vitest-environment jsdom
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import Page from './+page.svelte';

const READY = { installed: true, running: true, state: 'ready', hub: '127.0.0.1:8443', link: { state: 'up', since: '2026-11-15T10:00:00Z' } };
const PASSWORD = 'correct horse battery';
const INVITE = 'anima-invite.1.aW52aXRl';

const mocks = vi.hoisted(() => ({
  shell: {
    readLocalHub: vi.fn(),
    installLocalHub: vi.fn(),
    readLocalAccount: vi.fn(),
    newTotp: vi.fn(),
    createLocalAccount: vi.fn(),
    saveHubKey: vi.fn(),
  },
  check: vi.fn(),
  writeAccessToken: vi.fn(),
  loadClientAt: vi.fn(),
  sent: [] as { operation: string; variables: unknown }[],
  stayAwake: vi.fn(),
}));

vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/desktop/local-hub/shell', async (importOriginal) => ({ ...(await importOriginal<typeof import('$lib/desktop/local-hub/shell')>()), ...mocks.shell }));
vi.mock('$lib/desktop/hub-listener/shell', () => ({ checkHubListener: mocks.check }));
vi.mock('$lib/desktop/token', () => ({ writeAccessToken: mocks.writeAccessToken, readAccessToken: () => Promise.resolve(null) }));
vi.mock('$lib/desktop/stay-awake/shell', () => ({ readStayAwake: mocks.stayAwake }));
vi.mock('$lib/access/navigation', () => ({ loadClientAt: mocks.loadClientAt }));
vi.mock('$lib/access/listener/check', () => ({ tokenFor: () => () => Promise.resolve(null) }));
// The hub the page signs in to answers Login and CreateLinkInvite and records what it was sent.
vi.mock('$lib/hub/hub', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/hub/hub')>()),
  createRequestHub: ({ address }: { address: unknown }) => ({
    address,
    request: (document: TypedDocumentNode, variables: unknown) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      mocks.sent.push({ operation, variables });
      const answers: Record<string, unknown> = {
        Login: { login: { token: 'tok-new' } },
        CreateLinkInvite: { createLinkInvite: { invite: INVITE, expiresAt: '2099-01-01T00:15:00Z' } },
      };
      return Promise.resolve({ ok: true, value: answers[operation] });
    },
  }),
}));

let app: ReturnType<typeof mount> | undefined;

async function settle(): Promise<void> {
  for (let round = 0; round < 4; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

function click(label: string): void {
  const found = [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === label);
  if (found === undefined) throw new Error(`no button ${label}`);
  found.click();
  flushSync();
}

function typeInto(field: Element | null | undefined, value: string): void {
  if (!(field instanceof HTMLInputElement)) throw new Error('no field');
  field.value = value;
  field.dispatchEvent(new Event('input', { bubbles: true }));
}

async function enterCode(code: string): Promise<void> {
  const event = new Event('paste', { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'clipboardData', { value: { getData: () => code } });
  document.querySelector('[role="group"] input')?.dispatchEvent(event);
  await settle();
}

const heading = () => document.querySelector('h2')?.textContent;

beforeEach(() => {
  installAnimations();
  mocks.shell.readLocalHub.mockReset().mockResolvedValueOnce({ ok: true, value: { installed: false, running: false } }).mockResolvedValue({ ok: true, value: READY });
  mocks.shell.installLocalHub.mockReset().mockResolvedValue({ ok: true, value: null });
  mocks.shell.readLocalAccount.mockReset().mockResolvedValue({ ok: true, value: null });
  mocks.shell.newTotp.mockReset().mockResolvedValue({ ok: true, value: { secret: 'JBSWY3DPEHPK3PXP', uri: 'otpauth://totp/Anima:ann?secret=JBSWY3DPEHPK3PXP' } });
  mocks.shell.createLocalAccount.mockReset().mockResolvedValue({ ok: true, value: null });
  mocks.shell.saveHubKey.mockReset().mockResolvedValue({ ok: true, value: true });
  mocks.check.mockReset().mockResolvedValue({ ok: true, listener: 'own' });
  mocks.writeAccessToken.mockReset().mockResolvedValue(undefined);
  mocks.loadClientAt.mockReset();
  mocks.stayAwake.mockReset().mockResolvedValue({ ok: true, status: { level: 'off', batteryFloor: null, held: 'off', pause: null, lid: 'ready' } });
  mocks.sent.length = 0;
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
});

it('FRN-002, FRN-004, F-10, AWK-005: install, account, authenticator, sign-in with the next code, key, invite, done', async () => {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  await settle();
  expect(mocks.shell.installLocalHub).toHaveBeenCalledOnce();
  expect(heading()).toBe('Make your account');

  const fields = document.querySelectorAll('form input');
  typeInto(fields[0], 'ann');
  typeInto(fields[1], PASSWORD);
  typeInto(fields[2], PASSWORD);
  click('Continue');
  await settle();
  expect(heading()).toBe('Add Anima.ing to your authenticator app');
  await enterCode('111111');
  expect(mocks.shell.createLocalAccount).toHaveBeenCalledWith({ name: 'ann', password: PASSWORD, secret: 'JBSWY3DPEHPK3PXP', code: '111111' });

  expect(heading()).toBe('Sign in to your hub');
  // The step heading keeps focus: the code boxes do not take it when they show (NAV-005).
  expect(document.activeElement).toBe(document.querySelector('h2'));
  expect(mocks.check).toHaveBeenCalledWith('http://127.0.0.1:8443');
  expect(localStorage.getItem('anima.hubAddress')).toBe('http://127.0.0.1:8443');
  expect(target.textContent).toContain('Wait for the next code');
  await enterCode('222222');
  expect(mocks.sent[0]).toEqual({ operation: 'Login', variables: { input: { username: 'ann', password: PASSWORD, totpCode: '222222' } } });
  expect(mocks.writeAccessToken.mock.calls[0]?.[1]).toBe('tok-new');

  expect(heading()).toBe('Save a copy of the hub key');
  click('Save a copy of the hub key');
  await settle();
  expect(heading()).toBe('Invite your next device');
  typeInto(document.querySelector('form input'), 'laptop');
  click('Make an invite');
  await settle();
  expect(mocks.sent.at(-1)).toEqual({ operation: 'CreateLinkInvite', variables: { name: 'laptop' } });
  expect(document.querySelector('svg[role="img"]')?.getAttribute('aria-label')).toBe('Invite for laptop');
  expect(target.querySelector('code.text')?.textContent).toBe(INVITE);
  expect(target.textContent).toContain('This computer is the hub. Its checks stop while the computer sleeps.');
  click('Done');
  expect(mocks.loadClientAt).toHaveBeenCalledWith('/devices');
});

it('F-10: when another program holds the port nothing is saved and nothing is sent', async () => {
  mocks.shell.readLocalAccount.mockResolvedValue({ ok: true, value: { name: 'ann', totp: true } });
  mocks.check.mockResolvedValue({ ok: true, listener: 'other' });
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  await settle();
  expect(heading()).toBe('Sign in to your hub');
  expect(target.querySelector('[role="alert"]')?.textContent).toContain("Another program holds the hub's port");
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  expect(mocks.sent).toEqual([]);
});

it('FRN-003: an account that already exists signs in here with its name, password and code, then goes on to the key', async () => {
  mocks.shell.readLocalAccount.mockResolvedValue({ ok: true, value: { name: 'ann', totp: true } });
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  await settle();
  expect(heading()).toBe('Sign in to your hub');
  const fields = document.querySelectorAll('form input');
  typeInto(fields[0], 'ann');
  typeInto(fields[1], PASSWORD);
  click('Continue');
  await settle();
  await enterCode('333333');
  expect(mocks.sent[0]).toEqual({ operation: 'Login', variables: { input: { username: 'ann', password: PASSWORD, totpCode: '333333' } } });
  expect(heading()).toBe('Save a copy of the hub key');
  expect(mocks.loadClientAt).not.toHaveBeenCalled();
});
