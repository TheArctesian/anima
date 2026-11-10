// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub, type SentRequest } from '$lib/access/access.fixture';
import { UpdaterKeyError } from '$lib/desktop/builder/updater-key';
import type { Hub } from '$lib/hub/hub';
import { ThisBuilder } from '../builder.svelte';
import BuildsRow from '../BuildsRow.svelte';
import { BUILDER_TOKEN, FakeBuildsApi, FakeShell, ok, sleep } from './builds.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ builder: undefined as unknown, hub: undefined as unknown, shell: undefined as unknown }));
vi.mock('../builder.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../builder.svelte')>()),
  useThisBuilder: () => current.builder,
}));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/desktop/builder/shell', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/builder/shell')>()),
  desktopBuilderShell: { checkTools: () => (current.shell as FakeShell).checkTools() },
}));
const key = vi.hoisted(() => ({ read: vi.fn(), import: vi.fn(), unlock: vi.fn() }));
vi.mock('$lib/desktop/builder/updater-key', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/builder/updater-key')>()),
  readUpdaterKey: key.read,
  importUpdaterKey: key.import,
  unlockUpdaterKey: key.unlock,
}));

const BUILDERS = [
  { id: 'b-1', name: 'pop-os', platform: { os: 'LINUX', arch: 'X86_64' }, online: true },
  { id: 'b-2', name: 'studio', platform: { os: 'MACOS', arch: 'AARCH64' }, online: false },
];

let app: ReturnType<typeof mount> | undefined;
let shell: FakeShell;
let sent: SentRequest[];
let tokens: { token: string | null; ids: string | null };

beforeEach(() => {
  shell = new FakeShell(sleep);
  current.shell = shell;
  tokens = { token: null, ids: null };
  key.read.mockReset().mockResolvedValue({ present: false, unlocked: false, publicKey: null, matchesApp: null });
  key.import.mockReset();
  key.unlock.mockReset();
  const answers = { Builders: ok({ builders: BUILDERS }), RevokeBuilder: ok({ revokeBuilder: 'b-2' }) };
  const faked = fakeHub(answers as never);
  sent = faked.sent;
  current.hub = faked.hub satisfies Hub;
  current.builder = new ThisBuilder({
    operator: { enrol: () => Promise.resolve(ok({ builderId: 'b-1', token: BUILDER_TOKEN })), revoke: () => Promise.resolve(ok('b-1')) },
    builder: new FakeBuildsApi(),
    shell,
    tokens: {
      read: () => Promise.resolve(tokens.token),
      write: (token) => Promise.resolve(void (tokens.token = token)),
      clear: () => Promise.resolve(void (tokens.token = null)),
    },
    ids: { read: () => tokens.ids, write: (id) => void (tokens.ids = id), clear: () => void (tokens.ids = null) },
    operatorToken: () => Promise.resolve(null),
    lockKey: () => Promise.resolve(),
    computerName: () => Promise.resolve('pop-os'),
    sleep: () => new Promise(() => undefined),
    now: () => new Date(0),
  });
});

afterEach(async () => {
  await (current.builder as ThisBuilder).stop();
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

const settle = async () => {
  for (let round = 0; round < 40; round += 1) await Promise.resolve();
  flushSync();
};

async function show(): Promise<void> {
  await (current.builder as ThisBuilder).resume();
  app = mount(BuildsRow, { target: document.body });
  await settle();
}

const text = () => document.body.textContent.replace(/\s+/g, ' ');
const toggle = () => document.querySelector<HTMLInputElement>('input[type="checkbox"]');

it('BLD-020: shows the toggle off by default, the platform and the updater key state', async () => {
  await show();
  expect(toggle()?.checked).toBe(false);
  expect(text()).toContain('Build releases on this computer');
  expect(text()).toContain('Linux, x86-64');
  expect(text()).toContain('No updater key is stored on this computer.');
});

it('BLD-028, I-7: lists each missing tool under "Missing:"; a locked key is not one', async () => {
  shell.missing(['postgres', 'tauri-packages', 'updater-key-locked']);
  await show();
  const list = document.querySelector('ul.missing');
  expect(document.getElementById(list?.getAttribute('aria-labelledby') ?? '')?.textContent).toBe('Missing:');
  const items = [...(list?.querySelectorAll('li') ?? [])].map((li) => li.textContent.trim());
  expect(items).toEqual(['PostgreSQL 17', 'the Tauri system packages']);
});

it('BLD-021, BLD-023: turning the toggle on enrols and keeps the token in the key store', async () => {
  await show();
  const box = toggle();
  if (box === null) throw new Error('no toggle');
  box.checked = true;
  box.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();
  expect(tokens.token).toBe(BUILDER_TOKEN);
  expect(toggle()?.checked).toBe(true);
});

it('BLD-025: lists all builders and revokes the one the operator picks', async () => {
  await show();
  expect(text()).toContain('pop-os');
  expect(text()).toContain('macOS, Apple silicon (arm64), offline');
  document.querySelector<HTMLButtonElement>('button[aria-label="Revoke studio"]')?.click();
  await settle();
  expect(sent.filter((request) => request.operation === 'RevokeBuilder')).toEqual([{ operation: 'RevokeBuilder', variables: { id: 'b-2' } }]);
});

it('BLD-045: import sends the typed password, clears the field on success and shows the new state', async () => {
  key.import.mockResolvedValue({ present: true, unlocked: true, publicKey: 'RWQpublic', matchesApp: true });
  await show();
  const field = document.querySelector<HTMLInputElement>('input[type="password"]');
  if (field === null) throw new Error('no password field');
  field.value = 'pw';
  field.dispatchEvent(new Event('input'));
  [...document.querySelectorAll('button')].find((button) => button.textContent.includes('Import updater key'))?.click();
  await settle();
  expect(key.import).toHaveBeenCalledWith('pw', null);
  expect(field.value).toBe('');
  expect(text()).toContain('The updater key is unlocked until the app quits or building is turned off.');
  expect(text()).toContain('RWQpublic');
});

const button = (label: string) => [...document.querySelectorAll('button')].find((each) => each.textContent.includes(label));

function type(field: HTMLInputElement | null, value: string): void {
  if (field === null) throw new Error('no field');
  field.value = value;
  field.dispatchEvent(new Event('input'));
}

it('D-19: a locked key offers the unlock field, and a wrong password marks the field, names it and takes the focus', async () => {
  key.read.mockResolvedValue({ present: true, unlocked: false, publicKey: 'RWQpublic', matchesApp: true });
  key.unlock.mockRejectedValueOnce(new UpdaterKeyError('password'));
  await show();
  expect(text()).toContain('The updater key is stored and locked. Unlock it to build releases.');
  const field = document.querySelector<HTMLInputElement>('#updater-key-unlock');
  type(field, 'wrong');
  button('Unlock updater key')?.click();
  await settle();
  expect(key.unlock).toHaveBeenCalledWith('wrong');
  const alert = document.getElementById(field?.getAttribute('aria-describedby') ?? '');
  expect(alert?.getAttribute('role')).toBe('alert');
  expect(alert?.textContent).toBe('The password does not open the key.');
  expect(field?.getAttribute('aria-invalid')).toBe('true');
  expect(document.activeElement).toBe(field);
  key.unlock.mockResolvedValue({ present: true, unlocked: true, publicKey: 'RWQpublic', matchesApp: true });
  button('Unlock updater key')?.click();
  await settle();
  expect(document.querySelector('#updater-key-unlock')).toBeNull();
  expect(text()).toContain('The updater key is unlocked');
});

it('D-19, I-2: a key file without a password asks for a new one and moves the focus to it', async () => {
  key.import.mockRejectedValueOnce(new UpdaterKeyError('new-password-needed'));
  await show();
  button('Import updater key')?.click();
  await settle();
  const fresh = document.querySelector<HTMLInputElement>('#updater-key-new-password');
  expect(document.activeElement).toBe(fresh);
  expect(fresh?.getAttribute('aria-invalid')).toBe('true');
  key.import.mockResolvedValue({ present: true, unlocked: true, publicKey: 'RWQpublic', matchesApp: true });
  type(fresh, 'new-pw');
  button('Import updater key')?.click();
  await settle();
  expect(key.import).toHaveBeenLastCalledWith(null, 'new-pw');
});

it('I-2: the file password field takes the focus when the key file needs its password', async () => {
  key.import.mockRejectedValueOnce(new UpdaterKeyError('password-needed'));
  await show();
  button('Import updater key')?.click();
  await settle();
  const field = document.querySelector<HTMLInputElement>('#updater-key-password');
  expect(document.activeElement).toBe(field);
  expect(field?.getAttribute('aria-describedby')).toBe('updater-key-problem');
});

it('I-3: while busy the controls stay focusable, say they are disabled and ignore activation', async () => {
  let finish = (): void => undefined;
  key.import.mockReturnValue(new Promise((resolve) => (finish = () => resolve({ present: false, unlocked: false, publicKey: null, matchesApp: null }))));
  await show();
  const importButton = button('Import updater key');
  importButton?.click();
  await settle();
  expect(importButton?.hasAttribute('disabled')).toBe(false);
  expect(importButton?.getAttribute('aria-disabled')).toBe('true');
  importButton?.click();
  await settle();
  expect(key.import).toHaveBeenCalledTimes(1);
  finish();
  await settle();
  expect(importButton?.getAttribute('aria-disabled')).toBe('false');
});

it('I-3, I-4: while enrolling the toggle ignores clicks and the row says "Enrolling…"', async () => {
  await show();
  const builder = current.builder as ThisBuilder;
  builder.busy = true;
  flushSync();
  const box = toggle();
  expect(box?.getAttribute('aria-disabled')).toBe('true');
  box?.click();
  await settle();
  expect(box?.checked).toBe(false);
  expect(tokens.token).toBeNull();
  expect(text()).toContain('Enrolling…');
  builder.busy = false;
});

it('S-5, I-2: a builder problem is tied to the toggle', async () => {
  await show();
  const builder = current.builder as ThisBuilder;
  builder.problem = 'The hub did not enrol this computer as a builder.';
  flushSync();
  expect(document.getElementById(toggle()?.getAttribute('aria-describedby') ?? '')?.textContent).toBe(builder.problem);
});

it('I-5: until the list is read, no "No computer builds releases." line shows', async () => {
  let answer = (): void => undefined;
  const builders = new Promise((resolve) => (answer = () => resolve(ok({ builders: [] }))));
  current.hub = fakeHub({ Builders: builders } as never).hub;
  await show();
  expect(text()).not.toContain('No computer builds releases.');
  expect(document.querySelectorAll('.builders li')).toHaveLength(1);
  answer();
  await settle();
  expect(text()).toContain('No computer builds releases.');
});

it('D-19: warns when other programs can read the app memory', async () => {
  shell.tools = { ...shell.tools, memoryGuarded: false };
  await show();
  expect(text()).toContain('kernel.yama.ptrace_scope');
});
