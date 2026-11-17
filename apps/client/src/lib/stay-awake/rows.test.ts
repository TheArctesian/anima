// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { StayAwakeResult, StayAwakeStatus } from '$lib/desktop/stay-awake/shell';
import type { StayAwakeShell } from './stay-awake.svelte';
import StayAwakeRows from './StayAwakeRows.svelte';

const mocks = vi.hoisted(() => ({ origin: 'https://hub.anima.ing' }));
vi.mock('$lib/hub/context', () => ({ useHub: () => ({ address: { origin: mocks.origin } }) }));

const OFF: StayAwakeStatus = { level: 'off', batteryFloor: null, held: 'off', pause: null, lid: 'needs-set-up' };
const READY: StayAwakeStatus = { ...OFF, lid: 'ready' };

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  mocks.origin = 'https://hub.anima.ing';
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

/** A shell whose calls a test can replace. */
type Fake = { -readonly [Call in keyof StayAwakeShell]: StayAwakeShell[Call] } & {
  readonly calls: string[];
  /** The answer of the next change; without one, the change is applied to the status. */
  next: StayAwakeResult | null;
  announce: (status: StayAwakeStatus) => void;
  stopped: boolean;
};

function fake(start: StayAwakeStatus): Fake {
  let status = start;
  const answer = (name: string, change: Partial<StayAwakeStatus>): Promise<StayAwakeResult> => {
    shell.calls.push(name);
    if (shell.next !== null) return Promise.resolve(shell.next);
    status = { ...status, ...change };
    return Promise.resolve({ ok: true, status });
  };
  const shell: Fake = {
    calls: [],
    next: null,
    stopped: false,
    announce: () => undefined,
    read: () => Promise.resolve({ ok: true, status }),
    choose: (level) => answer(`choose ${level}`, { level, held: level }),
    setFloor: (floor) => answer(`floor ${String(floor)}`, { batteryFloor: floor }),
    setUpLid: () => answer('set up', { lid: 'ready' }),
    removeLidSetUp: () => answer('remove', { lid: 'needs-set-up' }),
    onChange: (listener) => {
      shell.announce = listener;
      return Promise.resolve(() => {
        shell.stopped = true;
      });
    },
  };
  return shell;
}

const settle = async () => {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
};

async function show(shell: Fake, mac = true, localHub: string | null = null): Promise<void> {
  app = mount(StayAwakeRows, { target: document.body, props: { shell, mac, localHub } });
  await settle();
}

const select = (id: string) => document.querySelector<HTMLSelectElement>(`#${id}`) as HTMLSelectElement;
const button = (name: string) => [...document.querySelectorAll('button')].find((each) => each.textContent.trim() === name);
const state = () => document.querySelector('[aria-live="polite"]')?.textContent.replace(/\s+/g, ' ').trim();
const alert = () => document.querySelector('[role="alert"]')?.textContent;

async function pick(id: string, value: string): Promise<void> {
  const control = select(id);
  control.value = value;
  control.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();
}

it('AWK-001: shows the three states with the saved one chosen, and says what the computer does', async () => {
  await show(fake({ ...READY, level: 'awake', held: 'awake' }));
  expect([...select('stay-awake-level').options].map((option) => option.textContent)).toEqual(['Off', 'Keep awake', 'Keep awake with the lid closed']);
  expect(select('stay-awake-level').value).toBe('awake');
  expect(state()).toBe('No idle sleep. The display can sleep.');
});

it('AWK-001: a choice goes to the shell at once and the row shows the new state', async () => {
  const shell = fake(READY);
  await show(shell);
  await pick('stay-awake-level', 'lid-closed');
  expect(shell.calls).toEqual(['choose lid-closed']);
  expect(state()).toContain('Stays on with the lid closed');
});

it('AWK-003: on a Mac the lid-closed state cannot be chosen before the set-up step, which the operator starts', async () => {
  const shell = fake(OFF);
  await show(shell);
  const lidClosed = () => select('stay-awake-level').options[2] as HTMLOptionElement;
  expect(lidClosed().disabled).toBe(true);
  expect(shell.calls).toEqual([]);
  button('Set up')?.click();
  await settle();
  expect(shell.calls).toEqual(['set up']);
  expect(lidClosed().disabled).toBe(false);
  button('Remove')?.click();
  await settle();
  expect(shell.calls).toEqual(['set up', 'remove']);
  expect(lidClosed().disabled).toBe(true);
});

it('AWK-003: a cancelled administrator step says so and changes nothing', async () => {
  const shell = fake(OFF);
  shell.next = { ok: false, problem: 'refused' };
  await show(shell);
  button('Set up')?.click();
  await settle();
  expect(alert()).toBe('Administrator step cancelled or unfinished. Nothing changed.');
  expect(button('Set up')).toBeDefined();
});

it('AWK-003: Linux has no set-up row, and a desktop that decides the lid itself says so', async () => {
  await show(fake({ ...OFF, lid: 'unsupported' }), false);
  expect(button('Set up')).toBeUndefined();
  expect(button('Remove')).toBeUndefined();
  expect((select('stay-awake-level').options[2] as HTMLOptionElement).disabled).toBe(true);
  expect(state()).toContain('This desktop controls the closed lid.');
});

it('AWK-006: the operator sets and removes the battery floor', async () => {
  const shell = fake(READY);
  await show(shell);
  expect(select('stay-awake-floor').value).toBe('none');
  await pick('stay-awake-floor', '20');
  await pick('stay-awake-floor', 'none');
  expect(shell.calls).toEqual(['floor 20', 'floor null']);
});

it('AWK-004, AWK-006: a change from the tray item or the battery shows without a reload', async () => {
  const shell = fake({ ...READY, level: 'awake', held: 'awake', batteryFloor: 20 });
  await show(shell);
  shell.announce({ ...READY, level: 'awake', held: 'off', batteryFloor: 20, pause: 'low-battery' });
  flushSync();
  expect(state()).toBe('Paused: battery below 20%. Resumes on mains power.');
  shell.announce({ ...READY, level: 'lid-closed', held: 'lid-closed', batteryFloor: 20 });
  flushSync();
  expect(select('stay-awake-level').value).toBe('lid-closed');
});

it('AWK-005: when the hub is this computer and nothing keeps it awake, the row says that checks stop during sleep', async () => {
  mocks.origin = 'http://127.0.0.1:8443';
  const shell = fake(READY);
  await show(shell, true, 'http://127.0.0.1:8443');
  const notice = () => document.querySelector('[role="status"]')?.textContent;
  expect(notice()).toBe('This computer is the hub: checks stop while it sleeps.');
  await pick('stay-awake-level', 'awake');
  expect(notice()).toBe('');
});

it('AWK-005, FRN-008: a loopback hub that is not the one run here gets no such line', async () => {
  mocks.origin = 'http://127.0.0.1:8443';
  await show(fake(READY));
  expect(document.querySelector('[role="status"]')).toBeNull();
});

it('AWK-005: a hub on a different computer gets no such line', async () => {
  await show(fake(READY));
  expect(document.querySelector('[role="status"]')).toBeNull();
});

it('a failed change shows fixed text, and the rows stop listening when they go', async () => {
  const shell = fake(READY);
  shell.next = { ok: false, problem: 'unsaved' };
  await show(shell);
  await pick('stay-awake-level', 'awake');
  expect(alert()).toBe('Not saved. Nothing changed.');
  expect(select('stay-awake-level').value).toBe('off');
  shell.next = null;
  await pick('stay-awake-level', 'awake');
  expect(alert()).toBe('');
  if (app) await unmount(app);
  app = undefined;
  expect(shell.stopped).toBe(true);
});

it('AWK-006: a refused floor goes back to the saved one, and a saved floor outside the steps is still shown', async () => {
  const shell = fake({ ...READY, batteryFloor: 15 });
  await show(shell);
  expect(select('stay-awake-floor').value).toBe('15');
  shell.next = { ok: false, problem: 'unsaved' };
  await pick('stay-awake-floor', '40');
  expect(select('stay-awake-floor').value).toBe('15');
});

it('AWK-004: a state announced while a change is on its way is not lost: the row asks the shell which is true', async () => {
  const shell = fake(READY);
  await show(shell);
  let answer: (result: StayAwakeResult) => void = () => undefined;
  shell.choose = () => new Promise((resolve) => (answer = resolve));
  const picked = pick('stay-awake-level', 'awake');
  flushSync();
  expect(select('stay-awake-level').disabled).toBe(true);
  const fromTray: StayAwakeStatus = { ...READY, level: 'lid-closed', held: 'lid-closed' };
  shell.read = () => Promise.resolve({ ok: true, status: fromTray });
  shell.announce(fromTray);
  answer({ ok: true, status: { ...READY, level: 'awake', held: 'awake' } });
  await picked;
  await settle();
  expect(select('stay-awake-level').value).toBe('lid-closed');
  expect(select('stay-awake-level').disabled).toBe(false);
});

it('rows that go before the shell starts to listen still stop listening', async () => {
  const shell = fake(READY);
  let listening: (stop: () => void) => void = () => undefined;
  shell.onChange = () => new Promise((resolve) => (listening = resolve));
  await show(shell);
  if (app) await unmount(app);
  app = undefined;
  let stopped = false;
  listening(() => (stopped = true));
  await settle();
  expect(stopped).toBe(true);
});
