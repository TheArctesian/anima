import { describe, expect, it, vi } from 'vitest';
import type { CloudDevice } from '$lib/desktop/cloud/device';
import type { HubResult } from '$lib/hub/hub';
import type { HubDevice } from './here';
import { HUB_NOT_TOLD, HUB_STILL_LISTS, leaveDaemonHere, NOT_REMOVED, type DaemonLeaveParts } from './leave';
import type { DaemonRemoval } from './shell';

const ADDRESS = 'fd41:4e49:4d41::7';
const MAC: CloudDevice = { name: 'stephen-s-macbook-pro', computer: "Stephen's MacBook Pro", os: 'MACOS' };
const HUB_DEVICES: readonly HubDevice[] = [
  { id: 'hub', host: 'localhost', metricsSource: 'DAEMON' },
  { id: 'mac', host: ADDRESS, metricsSource: 'DAEMON' },
];

const ok = <T>(value: T): HubResult<T> => ({ ok: true, value });
const unreachable: HubResult<never> = { ok: false, error: { code: 'unreachable', message: 'x' } };

interface Answers {
  readonly removal?: DaemonRemoval;
  readonly linkDevices?: HubResult<readonly { readonly name: string; readonly address: string }[]>;
  readonly devices?: HubResult<readonly HubDevice[]>;
  readonly revoke?: HubResult<unknown>;
}

/** A hosted hub that lists this Mac with its daemon, and a shell that removes a daemon that was here. */
function parts(answers: Answers = {}) {
  return {
    device: vi.fn(() => Promise.resolve<CloudDevice | null>(MAC)),
    linkDevices: vi.fn(() => Promise.resolve(answers.linkDevices ?? ok([{ name: MAC.name, address: ADDRESS }]))),
    devices: vi.fn(() => Promise.resolve(answers.devices ?? ok(HUB_DEVICES))),
    remove: vi.fn<DaemonLeaveParts['remove']>(() => Promise.resolve(answers.removal ?? { ok: true, wasHere: true })),
    revoke: vi.fn<DaemonLeaveParts['revoke']>(() => Promise.resolve(answers.revoke ?? ok({}))),
    log: vi.fn<(line: string) => void>(),
  };
}

describe('leaveDaemonHere (HMD-056)', () => {
  it('HMD-056: removes the daemon here, then revokes the device of this computer, and says nothing', async () => {
    const given = parts();
    expect(await leaveDaemonHere(given)).toBeNull();
    expect(given.remove).toHaveBeenCalledOnce();
    expect(given.revoke).toHaveBeenCalledWith('mac');
    expect(given.remove.mock.invocationCallOrder[0]).toBeLessThan(given.revoke.mock.invocationCallOrder[0] ?? 0);
  });

  it('HMD-056: a hub that cannot be reached still lets the removal here run, and the hub still lists the daemon', async () => {
    for (const answers of [{ linkDevices: unreachable }, { devices: unreachable }, { revoke: unreachable }]) {
      const given = parts(answers);
      expect(await leaveDaemonHere(given)).toBe(HUB_STILL_LISTS);
      expect(given.remove).toHaveBeenCalledOnce();
    }
    expect(HUB_STILL_LISTS).toContain('still lists it');
  });

  it('HMD-056: a request that throws is a hub that was not told', async () => {
    const given = parts();
    given.revoke.mockRejectedValue(new Error('offline'));
    expect(await leaveDaemonHere(given)).toBe(HUB_STILL_LISTS);
  });

  it('HMD-056: with no daemon here, an unreachable hub says nothing, so a second sign-out goes on', async () => {
    const given = parts({ removal: { ok: true, wasHere: false }, revoke: unreachable });
    expect(await leaveDaemonHere(given)).toBeNull();
  });

  it('HMD-056: a computer that is not a device of the hub has nothing to revoke', async () => {
    const given = parts({ devices: ok([HUB_DEVICES[0] as HubDevice]) });
    expect(await leaveDaemonHere(given)).toBeNull();
    expect(given.revoke).not.toHaveBeenCalled();
  });

  it('HMD-056: a removal that failed says so with the reason of the shell, and the hub too when it was not told', async () => {
    const removal: DaemonRemoval = { ok: false, problem: 'uninstall', message: 'The daemon did not take its unit and its token off this computer.' };
    expect(await leaveDaemonHere(parts({ removal }))).toBe(`${NOT_REMOVED} ${removal.message}`);
    const both = await leaveDaemonHere(parts({ removal, revoke: unreachable }));
    expect(both).toBe(`${NOT_REMOVED} ${removal.message} ${HUB_NOT_TOLD}`);
  });

  it('LOG-007: each log line is fixed text and codes, with no name, address or device id', async () => {
    const given = parts({ revoke: unreachable });
    await leaveDaemonHere(given);
    const lines = given.log.mock.calls.map(([line]) => line).join('\n');
    expect(lines).toContain('the hub did not revoke it (unreachable)');
    for (const secret of [MAC.name, ADDRESS, "'mac'"]) expect(lines).not.toContain(secret);
  });
});
