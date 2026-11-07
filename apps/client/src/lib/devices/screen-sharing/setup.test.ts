import { describe, expect, it, vi } from 'vitest';
import { SystemSettingsError } from '$lib/desktop/systemSettings';
import type { DeviceDetail } from '$lib/devices/api';
import { fakeCalls, answer, MAC, NAS, settle, STUDIO, UPDATED } from './calls.fixture';
import { SetupFlow } from './flow';
import { isThisComputer, NO_ANSWER_TEXT, offersScreenSharing, type SetupState } from './setup';

function run(over: Parameters<typeof fakeCalls>[0] = {}) {
  const fake = fakeCalls(over);
  const states: SetupState[] = [];
  const ready = vi.fn<(device: DeviceDetail) => void>();
  const flow = new SetupFlow(
    fake.calls,
    (state) => {
      states.push(state);
    },
    ready,
  );
  return { ...fake, flow, ready, states, last: () => states.at(-1) };
}

describe('offersScreenSharing (R-079)', () => {
  it.each([
    ['MACOS', false, true],
    ['LINUX', false, true],
    ['WINDOWS', false, false],
    ['OTHER', false, false],
    ['MACOS', true, false],
    ['LINUX', true, false],
  ] as const)('R-079: %s with VNC on=%s offers it: %s', (os, enabled, offered) => {
    expect(offersScreenSharing({ os, vnc: { enabled } })).toBe(offered);
  });
});

describe('isThisComputer (R-060)', () => {
  it('R-060: the device whose host is this computer VPN address is this computer', () => {
    expect(isThisComputer(STUDIO, MAC)).toBe(true);
    expect(isThisComputer({ host: ' 100.64.0.5 ' }, MAC)).toBe(true);
  });
  it('R-060: another host, or no desktop app, is not this computer', () => {
    expect(isThisComputer(NAS, MAC)).toBe(false);
    expect(isThisComputer(STUDIO, null)).toBe(false);
  });
});

describe('SetupFlow (R-079)', () => {
  it('R-079: a device that answers is tested on port 5900 with no credential, then VNC goes on with no password', async () => {
    const t = run();
    t.calls.test.mockResolvedValue(answer(true));
    await t.flow.start(NAS);
    expect(t.calls.test).toHaveBeenCalledWith({ host: '100.64.0.9', protocol: 'VNC', vnc: { enabled: true, port: 5900 } });
    expect(t.calls.update).toHaveBeenCalledWith('d2', { vnc: { enabled: true, port: 5900, username: null } });
    expect(t.ready).toHaveBeenCalledWith(UPDATED);
    expect(t.calls.openSharing).not.toHaveBeenCalled();
  });

  it('R-079: this computer, answering, has VNC turned on with its OS user as the username', async () => {
    const t = run();
    t.calls.test.mockResolvedValue(answer(true));
    await t.flow.start(STUDIO);
    expect(t.calls.update).toHaveBeenCalledWith('d1', { vnc: { enabled: true, port: 5900, username: 'ada' } });
  });

  it('R-079: this Mac not answering opens Sharing once, asks to turn on Screen Sharing and tests on the interval', async () => {
    const t = run();
    await t.flow.start(STUDIO);
    expect(t.calls.openSharing).toHaveBeenCalledTimes(1);
    expect(t.last()).toEqual({ kind: 'waiting', problem: null });
    expect(t.calls.test).toHaveBeenCalledTimes(1);
    t.clock.tick();
    await settle();
    t.clock.tick();
    await settle();
    expect(t.calls.test).toHaveBeenCalledTimes(3);
    expect(t.calls.openSharing).toHaveBeenCalledTimes(1);
    expect(t.calls.update).not.toHaveBeenCalled();
  });

  it('R-079: when this Mac answers at a later test, the flow carries on and hands the device over', async () => {
    const t = run();
    await t.flow.start(STUDIO);
    t.calls.test.mockResolvedValue(answer(true));
    t.clock.tick();
    await settle();
    expect(t.calls.update).toHaveBeenCalledWith('d1', { vnc: { enabled: true, port: 5900, username: 'ada' } });
    expect(t.ready).toHaveBeenCalledWith(UPDATED);
    expect(t.clock.waiting()).toBe(0);
  });

  it('R-079: Cancel stops the repeats', async () => {
    const t = run();
    await t.flow.start(STUDIO);
    t.flow.cancel();
    expect(t.clock.waiting()).toBe(0);
    t.clock.tick();
    await settle();
    expect(t.calls.test).toHaveBeenCalledTimes(1);
    expect(t.last()).toEqual({ kind: 'idle' });
  });

  it('R-079: an answer that arrives after Cancel does nothing', async () => {
    const t = run();
    await t.flow.start(STUDIO);
    let finish: (value: ReturnType<typeof answer>) => void = () => undefined;
    t.calls.test.mockReturnValueOnce(new Promise((resolve) => { finish = resolve; }));
    t.clock.tick();
    t.flow.cancel();
    finish(answer(true));
    await settle();
    expect(t.calls.update).not.toHaveBeenCalled();
  });

  it('R-079: disposing the flow, as unmount does, stops the repeats', async () => {
    const t = run();
    await t.flow.start(STUDIO);
    t.flow.dispose();
    expect(t.clock.waiting()).toBe(0);
  });

  it('R-079: if System Settings does not open, its fixed message shows and the flow keeps testing and can retry', async () => {
    const t = run();
    t.calls.openSharing.mockRejectedValueOnce(new SystemSettingsError('failed'));
    await t.flow.start(STUDIO);
    expect(t.last()).toEqual({ kind: 'waiting', problem: 'System Settings did not open.' });
    expect(t.clock.waiting()).toBe(1);
    await t.flow.reopenSharing();
    expect(t.calls.openSharing).toHaveBeenCalledTimes(2);
    expect(t.last()).toEqual({ kind: 'waiting', problem: null });
  });

  it('R-079: another device that does not answer gets the port 5900 text and no update', async () => {
    const t = run();
    await t.flow.start(NAS);
    expect(t.last()).toEqual({ kind: 'failed', message: NO_ANSWER_TEXT });
    expect(NO_ANSWER_TEXT).toMatch(/port 5900/);
    expect(NO_ANSWER_TEXT).toMatch(/Screen Sharing \(or the VNC server\) is off/);
    expect(NO_ANSWER_TEXT).toMatch(/VPN rule does not allow port 5900/);
    expect(t.calls.update).not.toHaveBeenCalled();
    expect(t.calls.openSharing).not.toHaveBeenCalled();
  });

  it('R-079: this computer outside the desktop app on a Mac gets the plain text and Sharing does not open', async () => {
    const t = run({ sharingAvailable: () => false });
    await t.flow.start(STUDIO);
    expect(t.last()).toEqual({ kind: 'failed', message: NO_ANSWER_TEXT });
    expect(t.calls.openSharing).not.toHaveBeenCalled();
  });

  it('R-079: a device that looks like this computer but is read as unknown is another device', async () => {
    const t = run({ thisMachine: () => Promise.resolve(null) });
    await t.flow.start(STUDIO);
    expect(t.last()).toEqual({ kind: 'failed', message: NO_ANSWER_TEXT });
  });

  it('R-079: a refused test shows the hub message and opens no session', async () => {
    const t = run();
    t.calls.test.mockResolvedValue({ ok: false, error: { code: 'VALIDATION', message: 'That host is not allowed.', fields: [] } });
    await t.flow.start(NAS);
    expect(t.last()).toEqual({ kind: 'failed', message: 'That host is not allowed.' });
    expect(t.calls.update).not.toHaveBeenCalled();
    expect(t.ready).not.toHaveBeenCalled();
  });

  it('R-079: a failed update shows its message and opens no session', async () => {
    const t = run();
    t.calls.test.mockResolvedValue(answer(true));
    t.calls.update.mockResolvedValue({ ok: false, error: { code: 'INTERNAL', message: 'The hub could not save.' } });
    await t.flow.start(NAS);
    expect(t.last()).toEqual({ kind: 'failed', message: 'The hub could not save.' });
    expect(t.ready).not.toHaveBeenCalled();
  });

  it('R-079: a hub error during a repeat ends the waiting with its message', async () => {
    const t = run();
    await t.flow.start(STUDIO);
    t.calls.test.mockResolvedValue({ ok: false, error: { code: 'unreachable', message: 'The hub did not answer. Check the VPN client.' } });
    t.clock.tick();
    await settle();
    expect(t.last()).toEqual({ kind: 'failed', message: 'The hub did not answer. Check the VPN client.' });
    expect(t.clock.waiting()).toBe(0);
  });
});
