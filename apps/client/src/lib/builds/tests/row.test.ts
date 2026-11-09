// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub, type SentRequest } from '$lib/access/access.fixture';
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
const key = vi.hoisted(() => ({ read: vi.fn(), import: vi.fn() }));
vi.mock('$lib/desktop/builder/updaterKey', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/builder/updaterKey')>()),
  readUpdaterKey: key.read,
  importUpdaterKey: key.import,
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
  key.read.mockReset().mockResolvedValue({ present: false, publicKey: null, matchesApp: null });
  key.import.mockReset();
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
    computerName: () => Promise.resolve('pop-os'),
    sleep: () => new Promise(() => undefined),
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

it('BLD-028: names each missing tool', async () => {
  shell.missing(['postgres', 'tauri-packages']);
  await show();
  const items = [...document.querySelectorAll('[aria-label="Missing tools"] li')].map((li) => li.textContent.trim());
  expect(items).toEqual(['PostgreSQL 17 is missing.', 'The Tauri system packages is missing.']);
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
  key.import.mockResolvedValue({ present: true, publicKey: 'RWQpublic', matchesApp: true });
  await show();
  const field = document.querySelector<HTMLInputElement>('input[type="password"]');
  if (field === null) throw new Error('no password field');
  field.value = 'pw';
  field.dispatchEvent(new Event('input'));
  [...document.querySelectorAll('button')].find((button) => button.textContent.includes('Import updater key'))?.click();
  await settle();
  expect(key.import).toHaveBeenCalledWith('pw');
  expect(field.value).toBe('');
  expect(text()).toContain('The updater key is stored on this computer.');
  expect(text()).toContain('RWQpublic');
});
