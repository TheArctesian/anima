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

it('BLD-020, R-116: shows the toggle off by default and the platform, and asks for no updater key', async () => {
  await show();
  expect(toggle()?.checked).toBe(false);
  expect(text()).toContain('Build releases on this computer');
  expect(text()).toContain('Linux, x86-64');
  expect(text()).not.toMatch(/updater key/i);
  expect(document.querySelector('input[type="password"]')).toBeNull();
});

it('BLD-028, I-7: lists each missing tool under "Missing:"', async () => {
  shell.missing(['cargo-nextest', 'postgres', 'tauri-packages']);
  await show();
  const list = document.querySelector('ul.missing');
  expect(document.getElementById(list?.getAttribute('aria-labelledby') ?? '')?.textContent).toBe('Missing:');
  const items = [...(list?.querySelectorAll('li') ?? [])].map((li) => li.textContent.trim());
  expect(items).toEqual(['cargo-nextest', 'PostgreSQL 17', 'the Tauri system packages']);
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

it('I-3: while a revoke runs the buttons stay focusable, say they are disabled and ignore activation', async () => {
  let answer = (): void => undefined;
  const revoked = new Promise((resolve) => (answer = () => resolve(ok({ revokeBuilder: 'b-2' }))));
  const faked = fakeHub({ Builders: ok({ builders: BUILDERS }), RevokeBuilder: revoked } as never);
  current.hub = faked.hub;
  await show();
  const revoke = document.querySelector<HTMLButtonElement>('button[aria-label="Revoke studio"]');
  revoke?.click();
  await settle();
  expect(revoke?.hasAttribute('disabled')).toBe(false);
  expect(revoke?.getAttribute('aria-disabled')).toBe('true');
  revoke?.click();
  await settle();
  expect(faked.sent.filter((request) => request.operation === 'RevokeBuilder')).toHaveLength(1);
  answer();
  await settle();
  expect(revoke?.getAttribute('aria-disabled')).toBe('false');
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
