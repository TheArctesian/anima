import { beforeEach, describe, expect, it, vi } from 'vitest';
import { agentBundled, installAgent, thisComputerShell } from './shell';

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke, isTauri: tauri.isTauri }));

const CODE = 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv';

beforeEach(() => {
  tauri.invoke.mockReset();
  tauri.isTauri.mockReturnValue(true);
});

describe('installAgent (MET-019, D-10 d)', () => {
  it('MET-019: passes only the code and the origin to the shell command', async () => {
    tauri.invoke.mockResolvedValue(null);
    expect(await installAgent(CODE, 'https://hub.test')).toEqual({ ok: true });
    expect(tauri.invoke).toHaveBeenCalledWith('agent_install', { code: CODE, origin: 'https://hub.test' });
  });

  it('LOG-007: a shell code becomes fixed text; the shell message is dropped', async () => {
    tauri.invoke.mockRejectedValue({ code: 'enrol', message: `raw ${CODE}` });
    const failed = await installAgent(CODE, 'https://hub.test');
    expect(failed).toMatchObject({ ok: false, problem: 'enrol' });
    expect(JSON.stringify(failed)).not.toContain(CODE);
    expect(JSON.stringify(failed)).not.toContain('raw');
  });

  it('MET-019: an unknown code, a string or nothing at all is `failed`', async () => {
    for (const reason of [{ code: 'toString' }, { code: 'mystery' }, 'command agent_install not found', null]) {
      tauri.invoke.mockRejectedValue(reason);
      expect(await installAgent(CODE, 'https://hub.test')).toMatchObject({ ok: false, problem: 'failed' });
    }
  });

  it('CLI-001: outside the desktop app nothing is invoked', async () => {
    tauri.isTauri.mockReturnValue(false);
    expect(await installAgent(CODE, 'https://hub.test')).toMatchObject({ ok: false, problem: 'unavailable' });
    expect(await agentBundled()).toBe(false);
    expect(thisComputerShell()).toBeNull();
    expect(tauri.invoke).not.toHaveBeenCalled();
  });
});

describe('agentBundled (D-10 d)', () => {
  it('D-10 d: only a plain `true` from the shell means the app carries the agent', async () => {
    for (const [answer, bundled] of [[true, true], [false, false], ['true', false], [null, false]] as const) {
      tauri.invoke.mockResolvedValue(answer);
      expect(await agentBundled()).toBe(bundled);
    }
    tauri.invoke.mockRejectedValue(new Error('no such command'));
    expect(await agentBundled()).toBe(false);
  });
});
