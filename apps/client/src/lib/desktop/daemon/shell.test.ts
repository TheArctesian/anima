import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { daemonBundled, installDaemon, removeDaemon, thisComputerShell } from './shell';

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke, isTauri: tauri.isTauri }));

const CODE = 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv';

beforeEach(() => {
  tauri.invoke.mockReset();
  tauri.isTauri.mockReturnValue(true);
});

describe('installDaemon (MET-019, D-10 d)', () => {
  it('MET-019: passes only the code and the origin to the shell command', async () => {
    tauri.invoke.mockResolvedValue(null);
    expect(await installDaemon(CODE, 'https://hub.test')).toEqual({ ok: true });
    expect(tauri.invoke).toHaveBeenCalledWith('daemon_install', { code: CODE, origin: 'https://hub.test' });
  });

  it('LOG-007: a shell code becomes fixed text; the shell message is dropped', async () => {
    tauri.invoke.mockRejectedValue({ code: 'enrol', message: `raw ${CODE}` });
    const failed = await installDaemon(CODE, 'https://hub.test');
    expect(failed).toMatchObject({ ok: false, problem: 'enrol' });
    expect(JSON.stringify(failed)).not.toContain(CODE);
    expect(JSON.stringify(failed)).not.toContain('raw');
  });

  it('MET-019: an unknown code, a string or nothing at all is `failed`', async () => {
    for (const reason of [{ code: 'toString' }, { code: 'mystery' }, 'command daemon_install not found', null]) {
      tauri.invoke.mockRejectedValue(reason);
      expect(await installDaemon(CODE, 'https://hub.test')).toMatchObject({ ok: false, problem: 'failed' });
    }
  });

  it('CLI-001: outside the desktop app nothing is invoked', async () => {
    tauri.isTauri.mockReturnValue(false);
    expect(await installDaemon(CODE, 'https://hub.test')).toMatchObject({ ok: false, problem: 'unavailable' });
    expect(await daemonBundled()).toBe(false);
    expect(thisComputerShell(fakeHub().hub)).toBeNull();
    expect(tauri.invoke).not.toHaveBeenCalled();
  });
});

describe('daemonBundled (D-10 d)', () => {
  it('D-10 d: only a plain `true` from the shell means the app carries the daemon', async () => {
    for (const [answer, bundled] of [[true, true], [false, false], ['true', false], [null, false]] as const) {
      tauri.invoke.mockResolvedValue(answer);
      expect(await daemonBundled()).toBe(bundled);
    }
    tauri.invoke.mockRejectedValue(new Error('no such command'));
    expect(await daemonBundled()).toBe(false);
  });
});

describe('removeDaemon (HMD-056)', () => {
  it('HMD-056: calls the shell with no argument and reads whether a copy was here', async () => {
    tauri.invoke.mockResolvedValue(true);
    expect(await removeDaemon()).toEqual({ ok: true, wasHere: true });
    expect(tauri.invoke).toHaveBeenCalledWith('daemon_remove');
    tauri.invoke.mockResolvedValue(false);
    expect(await removeDaemon()).toEqual({ ok: true, wasHere: false });
  });

  it('HMD-056: a refusal of the shell is its code and fixed words', async () => {
    tauri.invoke.mockRejectedValue({ code: 'uninstall', message: 'raw' });
    expect(await removeDaemon()).toEqual({
      ok: false,
      problem: 'uninstall',
      message: 'The daemon did not take its unit and its token off this computer.',
    });
  });

  it('CLI-001: outside the desktop app nothing is removed and nothing is invoked', async () => {
    tauri.isTauri.mockReturnValue(false);
    expect(await removeDaemon()).toEqual({ ok: true, wasHere: false });
    expect(tauri.invoke).not.toHaveBeenCalled();
  });
});
