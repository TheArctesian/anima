// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import type { CommandLineResult, CommandLineStatus } from '$lib/desktop/command-line/shell';
import type { CommandLineShell } from '../command-line.svelte';
import CommandLineRow from '../CommandLineRow.svelte';

const FOLDER = '/Users/ann/.local/bin';
const PATH = `${FOLDER}/anima`;
const NOT_INSTALLED: CommandLineStatus = {
  state: 'not-installed',
  folder: FOLDER,
  path: PATH,
  onPath: false,
  create: true,
  profileLine: 'export PATH="$HOME/.local/bin:$PATH"',
};

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

interface Fake extends CommandLineShell {
  readonly calls: string[];
  next: CommandLineResult | null;
}

function fake(status: CommandLineStatus): Fake {
  const ok = (value: CommandLineStatus): CommandLineResult => ({ ok: true, status: value });
  const shell: Fake = {
    calls: [],
    next: null,
    read: () => Promise.resolve(ok(status)),
    install: (folder) => {
      shell.calls.push(`install ${folder}`);
      return Promise.resolve(shell.next ?? ok({ state: 'installed', path: PATH }));
    },
    remove: () => {
      shell.calls.push('remove');
      return Promise.resolve(ok(NOT_INSTALLED));
    },
  };
  return shell;
}

const settle = async () => {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
};

const button = (name: string) => [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === name);
const words = () => document.querySelector('[aria-live="polite"]')?.textContent.replace(/\s+/g, ' ').trim();

async function show(shell: Fake): Promise<void> {
  app = mount(CommandLineRow, { target: document.body, props: { shell } });
  await settle();
}

it('INS-006: the row is named and says its state in words with the path, and the profile line when needed', async () => {
  await show(fake(NOT_INSTALLED));
  expect(document.body.textContent).toContain('Command line tool');
  expect(words()).toBe(`Not installed. Install makes the folder ${FOLDER} and puts anima at ${PATH}.`);
  expect(document.querySelector('code')?.textContent).toBe('export PATH="$HOME/.local/bin:$PATH"');
  expect(button('Install')?.disabled).toBe(false);
  expect(button('Remove')?.disabled).toBe(true);
});

it('INS-004: Install shows the exact path and moves focus to Confirm; only Confirm installs', async () => {
  const shell = fake(NOT_INSTALLED);
  await show(shell);
  button('Install')?.click();
  await settle();
  expect(words()).toBe(`Install anima at ${PATH}?`);
  expect(document.activeElement).toBe(button('Confirm'));
  expect(shell.calls).toEqual([]);
  button('Confirm')?.click();
  await settle();
  expect(shell.calls).toEqual([`install ${FOLDER}`]);
  expect(words()).toBe(`Installed at ${PATH}.`);
  expect(button('Remove')?.disabled).toBe(false);
});

it('INS-004: Cancel returns focus to Install and installs nothing', async () => {
  const shell = fake(NOT_INSTALLED);
  await show(shell);
  button('Install')?.click();
  await settle();
  button('Cancel')?.click();
  await settle();
  expect(document.activeElement).toBe(button('Install'));
  expect(shell.calls).toEqual([]);
});

it('INS-006: a refusal is an alert that names what is in the way', async () => {
  const shell = fake(NOT_INSTALLED);
  shell.next = { ok: false, problem: 'occupied', occupant: { kind: 'link', linkTarget: '/opt/x/anima' } };
  await show(shell);
  button('Install')?.click();
  await settle();
  button('Confirm')?.click();
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain(`There is a link at ${PATH} to /opt/x/anima.`);
});

it('INS-006: Remove deletes the link and the row says not installed', async () => {
  const shell = fake({ state: 'installed', path: PATH });
  await show(shell);
  expect(button('Install')?.disabled).toBe(true);
  button('Remove')?.click();
  await settle();
  expect(shell.calls).toEqual(['remove']);
  expect(words()).toContain('Not installed.');
});

it('INS-004: an app on a disk image says to move it, with both buttons off', async () => {
  await show(fake({ state: 'unavailable', reason: 'disk-image' }));
  expect(words()).toContain('Move Anima to Applications');
  expect(button('Install')?.disabled).toBe(true);
  expect(button('Remove')?.disabled).toBe(true);
});
