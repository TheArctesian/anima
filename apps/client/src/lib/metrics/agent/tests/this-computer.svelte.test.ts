// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { thisComputerShell } from '$lib/desktop/agent/shell';
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
    expect(alert).toContain('~/.local/bin, or a folder above it, can be changed by other users.');
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
