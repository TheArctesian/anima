import { beforeEach, describe, expect, it, vi } from 'vitest';
import { chooseLevel, onStayAwake, readStayAwake, removeLidSetUp, setBatteryFloor, setUpLid, statusOf, type StayAwakeStatus } from './stay-awake';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));
vi.mock('@tauri-apps/api/event', () => ({ listen: shell.listen }));

const STATUS: StayAwakeStatus = { level: 'lid-closed', batteryFloor: 20, held: 'awake', pause: 'lid-not-ready', lid: 'needs-set-up' };

beforeEach(() => {
  shell.invoke.mockReset().mockResolvedValue(STATUS);
  shell.listen.mockReset();
  shell.isTauri.mockReturnValue(true);
});

describe('statusOf', () => {
  it('AWK-001: takes the status the shell sends, with and without a floor and a pause', () => {
    expect(statusOf(STATUS)).toEqual(STATUS);
    const plain = { level: 'off', batteryFloor: null, held: 'off', pause: null, lid: 'ready' };
    expect(statusOf(plain)).toEqual(plain);
  });

  it('refuses anything that is not a status, so a wrong answer never shows as a state', () => {
    for (const wrong of [null, 'awake', {}, { ...STATUS, level: 'on' }, { ...STATUS, held: 1 }, { ...STATUS, pause: 'asleep' }, { ...STATUS, lid: 'open' }, { ...STATUS, batteryFloor: '20' }]) {
      expect(statusOf(wrong)).toBeNull();
    }
  });
});

describe('the shell calls', () => {
  it('AWK-001, AWK-006, AWK-003: each change names its command and passes only a level or a floor', async () => {
    await readStayAwake();
    await chooseLevel('awake');
    await setBatteryFloor(30);
    await setBatteryFloor(null);
    await setUpLid();
    await removeLidSetUp();
    expect(shell.invoke.mock.calls).toEqual([
      ['stay_awake_read', undefined],
      ['stay_awake_choose', { level: 'awake' }],
      ['stay_awake_floor', { floor: 30 }],
      ['stay_awake_floor', { floor: null }],
      ['stay_awake_lid_set_up', undefined],
      ['stay_awake_lid_remove', undefined],
    ]);
  });

  it('gives the status of a call that worked', async () => {
    expect(await chooseLevel('lid-closed')).toEqual({ ok: true, status: STATUS });
  });

  it("gives the shell's code for a refusal, and `failed` for any other answer", async () => {
    shell.invoke.mockRejectedValue({ code: 'refused', message: 'raw os text' });
    expect(await setUpLid()).toEqual({ ok: false, problem: 'refused' });
    shell.invoke.mockRejectedValue('command not found');
    expect(await setUpLid()).toEqual({ ok: false, problem: 'failed' });
    shell.invoke.mockResolvedValue({ level: 'on' });
    expect(await readStayAwake()).toEqual({ ok: false, problem: 'failed' });
  });

  it('outside the desktop app, calls nothing', async () => {
    shell.isTauri.mockReturnValue(false);
    expect(await chooseLevel('awake')).toEqual({ ok: false, problem: 'unsupported' });
    const stop = await onStayAwake(() => undefined);
    stop();
    expect(shell.invoke).not.toHaveBeenCalled();
    expect(shell.listen).not.toHaveBeenCalled();
  });
});

describe('onStayAwake', () => {
  it('AWK-004: hears each state the shell announces and drops a payload that is not one', async () => {
    const stop = vi.fn();
    shell.listen.mockResolvedValue(stop);
    const heard: StayAwakeStatus[] = [];
    const unlisten = await onStayAwake((status) => heard.push(status));
    const [[event, handler]] = shell.listen.mock.calls as [[string, (event: { payload: unknown }) => void]];
    expect(event).toBe('stay-awake');
    handler({ payload: STATUS });
    handler({ payload: { level: 'on' } });
    expect(heard).toEqual([STATUS]);
    unlisten();
    expect(stop).toHaveBeenCalledOnce();
  });
});
