// @vitest-environment jsdom
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { thisComputerShell, type ThisComputerShell } from '$lib/desktop/agent/shell';
import type { AgentCalls } from '../install.svelte';
import { ThisComputerInstall } from '../this-computer.svelte';
import { CODE, ENROLMENT, button, click, setUp, show, tearDown, text } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

/** The desktop shell, faked at the IPC edge: every shell command the panel reaches goes through this `invoke`. */
const tauri = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke, isTauri: tauri.isTauri }));

const HERE = '100.64.0.9';
const MACHINE = { name: 'nas', user: 'ada', os: 'LINUX', vpnAddress: HERE, sshListening: true };
const INSTALLED = { AgentStatus: { device: { id: 'd1', metricsSource: 'AGENT', metricsStale: false } } };

/** Answers the shell's commands; `agent_install` as `install` says. */
function shellAnswers(install: () => Promise<unknown>, bundled = true): void {
  tauri.invoke.mockImplementation((command: string) => {
    if (command === 'this_machine_read') return Promise.resolve(MACHINE);
    if (command === 'agent_bundled') return Promise.resolve(bundled);
    if (command === 'agent_install') return install();
    return Promise.reject(new Error(`unexpected ${command}`));
  });
}

const offered = () => [...document.querySelectorAll('button')].some((each) => each.textContent.trim() === 'Install on this computer');
const installCalls = () => tauri.invoke.mock.calls.filter(([command]) => command === 'agent_install');

beforeEach(() => {
  setUp();
  tauri.invoke.mockReset();
  tauri.isTauri.mockReturnValue(true);
});
afterEach(tearDown);

describe('Install agent on this computer, in the desktop app (MET-019, D-10 d)', () => {
  it('MET-019: the code goes from the hub to the shell with the origin, and is never shown or copied', async () => {
    shellAnswers(() => Promise.resolve(null));
    const { fake, copied } = await show({ ...ENROLMENT, ...INSTALLED }, { host: HERE, shell: thisComputerShell() });
    expect(offered()).toBe(true);
    await click('Install on this computer');
    expect(installCalls()).toEqual([['agent_install', { code: CODE, origin: 'https://hub.test' }]]);
    expect(fake.requests.map((each) => each.operation)).toEqual(['EnrolAgent', 'AgentStatus']);
    expect(text()).not.toContain(CODE);
    expect(document.querySelector('pre')).toBeNull();
    expect(copied).toEqual([]);
    expect(text()).toContain('The agent is installed and runs on this computer.');
    expect(text()).toContain('Metrics are current.');
  });

  it('LOG-007: a shell error shows fixed text, never the shell message or the code, and the command stays as the way round', async () => {
    shellAnswers(() => Promise.reject({ code: 'untrusted', message: `raw ${CODE}` }));
    await show(ENROLMENT, { host: HERE, shell: thisComputerShell() });
    await click('Install on this computer');
    const alert = document.querySelector('[role="alert"]')?.textContent ?? '';
    expect(alert).toContain('Run `chmod g-w,o-w` on it, and `setfacl -b` for an ACL');
    expect(text()).not.toContain(CODE);
    expect(text()).not.toContain('raw');
    expect(button('Show install command')).toBeDefined();
  });

  it('MET-019: a second press while the shell works does not make a second code', async () => {
    let finish: (value: unknown) => void = () => undefined;
    shellAnswers(() => new Promise((resolve) => { finish = resolve; }));
    const { fake } = await show({ ...ENROLMENT, ...INSTALLED }, { host: HERE, shell: thisComputerShell() });
    await click('Install on this computer');
    await click('Installing…');
    expect(fake.requests.filter((each) => each.operation === 'EnrolAgent')).toHaveLength(1);
    finish(null);
  });

  it('MET-019: a hub refusal of the code stops before the shell is asked', async () => {
    shellAnswers(() => Promise.resolve(null));
    await show({ EnrolAgent: { code: 'FORBIDDEN', message: 'Not allowed.' } }, { host: HERE, shell: thisComputerShell() });
    await click('Install on this computer');
    expect(installCalls()).toEqual([]);
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('Not allowed.');
  });
});

describe('Install agent: the command panel where the app cannot install (MET-019, D-10 b)', () => {
  it('R-060: another device gets only the command', async () => {
    shellAnswers(() => Promise.resolve(null));
    await show(ENROLMENT, { host: '100.64.0.10', shell: thisComputerShell() });
    expect(offered()).toBe(false);
    expect(button('Install agent')).toBeDefined();
    expect(tauri.invoke.mock.calls.map(([command]) => command)).not.toContain('agent_bundled');
  });

  it('D-10 d: an app that carries no agent gets only the command', async () => {
    shellAnswers(() => Promise.resolve(null), false);
    await show(ENROLMENT, { host: HERE, shell: thisComputerShell() });
    expect(offered()).toBe(false);
    await click('Install agent');
    expect(document.querySelector('pre')?.textContent).toContain('anima-agent');
    expect(installCalls()).toEqual([]);
  });

  it('CLI-001: in a browser the shell is never asked', async () => {
    tauri.isTauri.mockReturnValue(false);
    await show(ENROLMENT, { host: HERE, shell: thisComputerShell() });
    expect(offered()).toBe(false);
    expect(tauri.invoke).not.toHaveBeenCalled();
  });
});

const MACHINE_HERE = { ...MACHINE, os: 'LINUX' as const };
const DEVICE = { id: 'd1', host: HERE };
const CODE_MADE = { ok: true as const, value: { code: CODE, expiresAt: '2026-11-09T10:15:00Z' } };

/** A promise and the function that settles it. */
function pending<T>(): { promise: Promise<T>; settle: (value: T) => void } {
  let settle: (value: T) => void = () => undefined;
  const promise = new Promise<T>((resolve) => { settle = resolve; });
  return { promise, settle };
}

function shellOf(overrides: Partial<ThisComputerShell>): ThisComputerShell {
  return {
    thisMachine: () => Promise.resolve(MACHINE_HERE),
    bundled: () => Promise.resolve(true),
    install: () => Promise.resolve({ ok: true }),
    ...overrides,
  };
}

async function offeredInstall(calls: Pick<AgentCalls, 'enrolAgent'>, shell: ThisComputerShell): Promise<ThisComputerInstall> {
  const here = new ThisComputerInstall(calls, shell, DEVICE, 'https://hub.test');
  await here.check();
  expect(here.offered).toBe(true);
  return here;
}

describe('Install agent on this computer: nothing changes after the panel closes (MET-019, LOG-007)', () => {
  it('MET-019: closed while this computer is read: nothing is offered and the shell is asked no more', async () => {
    const machine = pending<typeof MACHINE_HERE>();
    const bundled = vi.fn(() => Promise.resolve(true));
    const here = new ThisComputerInstall({ enrolAgent: vi.fn() }, shellOf({ thisMachine: () => machine.promise, bundled }), DEVICE, 'https://hub.test');
    const checking = here.check();
    here.dispose();
    machine.settle(MACHINE_HERE);
    await checking;
    expect(here.offered).toBe(false);
    expect(bundled).not.toHaveBeenCalled();
  });

  it('MET-019: closed while the shell says whether it carries the agent: nothing is offered', async () => {
    const bundled = pending<boolean>();
    const here = new ThisComputerInstall({ enrolAgent: vi.fn() }, shellOf({ bundled: () => bundled.promise }), DEVICE, 'https://hub.test');
    const checking = here.check();
    await tick();
    here.dispose();
    bundled.settle(true);
    await checking;
    expect(here.offered).toBe(false);
  });

  it('LOG-007: closed while the code is made: the code never goes to the shell, and the state is unchanged', async () => {
    const made = pending<typeof CODE_MADE>();
    const install = vi.fn(() => Promise.resolve({ ok: true as const }));
    const here = await offeredInstall({ enrolAgent: () => made.promise }, shellOf({ install }));
    const installing = here.install();
    here.dispose();
    made.settle(CODE_MADE);
    expect(await installing).toBe(false);
    expect(install).not.toHaveBeenCalled();
    expect([here.installed, here.error]).toEqual([false, null]);
  });

  it('MET-019: closed while the shell installs: neither success nor failure is shown, and the caller is told no', async () => {
    for (const answer of [{ ok: true as const }, { ok: false as const, problem: 'enrol' as const, message: 'refused' }]) {
      const done = pending<typeof answer>();
      const here = await offeredInstall({ enrolAgent: () => Promise.resolve(CODE_MADE) }, shellOf({ install: () => done.promise }));
      const installing = here.install();
      await tick();
      here.dispose();
      done.settle(answer);
      expect(await installing).toBe(false);
      expect([here.installed, here.error]).toEqual([false, null]);
    }
  });

  it('MET-019: the panel closed during an install does not read the agent state afterwards', async () => {
    let finish: (value: unknown) => void = () => undefined;
    shellAnswers(() => new Promise((resolve) => { finish = resolve; }));
    const { fake } = await show({ ...ENROLMENT, ...INSTALLED }, { host: HERE, shell: thisComputerShell() });
    await click('Install on this computer');
    await tearDown();
    finish(null);
    await tick();
    await tick();
    expect(fake.requests.map((each) => each.operation)).toEqual(['EnrolAgent']);
  });
});
