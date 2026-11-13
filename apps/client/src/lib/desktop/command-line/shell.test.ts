import { beforeEach, describe, expect, it, vi } from 'vitest';
import { installCommandLine, readCommandLine, removeCommandLine, statusOf } from './shell';

const tauri = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke, isTauri: tauri.isTauri }));

const NOT_INSTALLED = {
  state: 'not-installed',
  folder: '/Users/ann/.local/bin',
  path: '/Users/ann/.local/bin/anima',
  onPath: false,
  create: true,
  profileLine: 'export PATH="$HOME/.local/bin:$PATH"',
};

beforeEach(() => {
  tauri.invoke.mockReset();
  tauri.isTauri.mockReturnValue(true);
});

describe('statusOf (INS-006)', () => {
  it('INS-006: reads each state the shell sends', () => {
    expect(statusOf(NOT_INSTALLED)).toEqual(NOT_INSTALLED);
    expect(statusOf({ state: 'installed', path: '/a/anima' })).toEqual({ state: 'installed', path: '/a/anima' });
    const elsewhere = { state: 'elsewhere', folder: '/a', path: '/a/anima', pointsAt: '/Old.app/Contents/MacOS/anima' };
    expect(statusOf(elsewhere)).toEqual(elsewhere);
    expect(statusOf({ state: 'unavailable', reason: 'disk-image' })).toEqual({ state: 'unavailable', reason: 'disk-image' });
  });

  it('INS-006: anything else is not a state', () => {
    for (const value of [
      null,
      'installed',
      { state: 'installed' },
      { state: 'unavailable', reason: 'toString' },
      { ...NOT_INSTALLED, onPath: 'no' },
      { ...NOT_INSTALLED, profileLine: 3 },
      { state: 'mystery', path: '/a' },
    ]) {
      expect(statusOf(value)).toBeNull();
    }
  });
});

describe('the shell calls (INS-004 to INS-006)', () => {
  it('INS-004: install passes only the folder the shell proposed', async () => {
    tauri.invoke.mockResolvedValue({ state: 'installed', path: '/Users/ann/.local/bin/anima' });
    expect(await installCommandLine('/Users/ann/.local/bin')).toEqual({
      ok: true,
      status: { state: 'installed', path: '/Users/ann/.local/bin/anima' },
    });
    expect(tauri.invoke).toHaveBeenCalledWith('command_line_install', { folder: '/Users/ann/.local/bin' });
  });

  it('INS-006: read and remove take no arguments', async () => {
    tauri.invoke.mockResolvedValue(NOT_INSTALLED);
    await readCommandLine();
    await removeCommandLine();
    expect(tauri.invoke.mock.calls).toEqual([['command_line_read', undefined], ['command_line_remove', undefined]]);
  });

  it('INS-006: a refusal keeps the code and what is in the way, never the shell text', async () => {
    tauri.invoke.mockRejectedValue({ code: 'occupied', message: 'raw shell text', occupant: 'link', linkTarget: '/opt/x/anima' });
    const result = await installCommandLine('/a');
    expect(result).toEqual({ ok: false, problem: 'occupied', occupant: { kind: 'link', linkTarget: '/opt/x/anima' } });
    expect(JSON.stringify(result)).not.toContain('raw shell text');
  });

  it('INS-006: an unknown code, a string, nothing or a bad answer is `failed`', async () => {
    for (const reason of [{ code: 'toString' }, 'command not found', null]) {
      tauri.invoke.mockRejectedValue(reason);
      expect(await removeCommandLine()).toEqual({ ok: false, problem: 'failed', occupant: null });
    }
    tauri.invoke.mockResolvedValue({ state: 'installed' });
    expect(await readCommandLine()).toEqual({ ok: false, problem: 'failed', occupant: null });
  });

  it('CLI-001: outside the desktop app nothing is invoked', async () => {
    tauri.isTauri.mockReturnValue(false);
    expect(await readCommandLine()).toMatchObject({ ok: false, problem: 'unsupported' });
    expect(tauri.invoke).not.toHaveBeenCalled();
  });
});
