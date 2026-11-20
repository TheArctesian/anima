import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CloudDevice } from '$lib/desktop/cloud/device';
import type { HubError, HubResult } from '$lib/hub/hub';
import { forgetDaemonHere, installDaemonHere, type DaemonHereParts, type HubDevice } from './here';

const ADDRESS = 'fd41:4e49:4d41::7';
const MAC: CloudDevice = { name: 'stephen-s-macbook-pro', computer: "Stephen's MacBook Pro", os: 'MACOS' };
const ON_LINK = [
  { name: 'pop-os', address: 'fd41:4e49:4d41::2' },
  { name: MAC.name, address: ADDRESS },
];
const HUB_DEVICES: readonly HubDevice[] = [
  { id: 'hub', host: 'localhost', metricsSource: 'DAEMON' },
  { id: 'mac', host: 'FD41:4E49:4D41:0:0:0:0:7', metricsSource: 'NONE' },
  { id: 'linux', host: 'fd41:4e49:4d41::2', metricsSource: 'NONE' },
];
const CODE = 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv';
const ORIGIN = 'http://127.0.0.1:41001';

const ok = <T>(value: T): HubResult<T> => ({ ok: true, value });
const denied: HubResult<never> = { ok: false, error: { code: 'FORBIDDEN', message: 'x' } satisfies HubError };

type Answers = { readonly [Part in Exclude<keyof DaemonHereParts, 'origin'>]?: Awaited<ReturnType<DaemonHereParts[Part]>> };

/** A hosted hub that lists this Mac with no daemon, and a build that carries the daemon. */
function parts(answers: Answers = {}) {
  return {
    bundled: vi.fn(() => Promise.resolve(answers.bundled ?? true)),
    device: vi.fn(() => Promise.resolve(answers.device === undefined ? MAC : answers.device)),
    linkDevices: vi.fn(() => Promise.resolve(answers.linkDevices ?? ok(ON_LINK))),
    devices: vi.fn(() => Promise.resolve(answers.devices ?? ok(HUB_DEVICES))),
    enrol: vi.fn<DaemonHereParts['enrol']>(() => Promise.resolve(answers.enrol ?? ok({ code: CODE }))),
    install: vi.fn<DaemonHereParts['install']>(() => Promise.resolve(answers.install ?? { ok: true })),
    origin: ORIGIN,
  };
}

beforeEach(() => {
  forgetDaemonHere();
});

describe('installDaemonHere (HMD-053, R-240)', () => {
  it('enrols the device of this computer, found by its link address, and has the shell install with the code', async () => {
    const hub = parts();
    expect(await installDaemonHere(hub)).toEqual({ kind: 'installed' });
    expect(hub.enrol).toHaveBeenCalledExactlyOnceWith('mac');
    expect(hub.install).toHaveBeenCalledExactlyOnceWith(CODE, ORIGIN);
  });

  it('does nothing when the daemon of this computer reports already', async () => {
    const reporting = HUB_DEVICES.map((device) => (device.id === 'mac' ? { ...device, metricsSource: 'DAEMON' as const } : device));
    const hub = parts({ devices: ok(reporting) });
    expect(await installDaemonHere(hub)).toEqual({ kind: 'present' });
    expect(hub.enrol).not.toHaveBeenCalled();
    expect(hub.install).not.toHaveBeenCalled();
  });

  it.each<[string, Answers]>([
    ['a build that carries no daemon', { bundled: false }],
    ['a shell that cannot read this computer', { device: null }],
    ['a computer that is not on the link', { linkDevices: ok([ON_LINK[0] as (typeof ON_LINK)[number]]) }],
    ['a link list that cannot be read', { linkDevices: denied }],
    ['a computer that is not a device of the hub yet', { devices: ok([HUB_DEVICES[0] as HubDevice]) }],
    ['a device list that cannot be read', { devices: denied }],
  ])('makes no code for %s', async (_name, answers) => {
    const hub = parts(answers);
    expect(await installDaemonHere(hub)).toEqual({ kind: 'not-ready' });
    expect(hub.enrol).not.toHaveBeenCalled();
    expect(hub.install).not.toHaveBeenCalled();
  });

  it('names the step that failed: the code of the hub, or the problem of the shell', async () => {
    const refused = parts({ enrol: denied });
    expect(await installDaemonHere(refused)).toEqual({ kind: 'failed', step: 'enrol' });
    expect(refused.install).not.toHaveBeenCalled();

    forgetDaemonHere();
    const untrusted = parts({ install: { ok: false, problem: 'untrusted', message: 'x' } });
    expect(await installDaemonHere(untrusted)).toEqual({ kind: 'failed', step: 'untrusted' });
    expect(untrusted.install).toHaveBeenCalledTimes(1);
  });

  it('tries once for each start of the app, and two pages that open together make one code', async () => {
    const hub = parts({ install: { ok: false, problem: 'enrol', message: 'x' } });
    const [first, second] = await Promise.all([installDaemonHere(hub), installDaemonHere(hub)]);
    expect(first).toEqual({ kind: 'failed', step: 'enrol' });
    expect(second).toEqual(first);
    expect(await installDaemonHere(hub)).toEqual({ kind: 'not-ready' });
    expect(hub.enrol).toHaveBeenCalledTimes(1);
  });

  it('a part that throws is a failure, not an exception of the page', async () => {
    const hub = parts();
    hub.devices.mockRejectedValueOnce(new Error('x'));
    expect(await installDaemonHere(hub)).toEqual({ kind: 'failed', step: 'failed' });
  });
});

describe('installDaemonHere while the link has no socket yet (HMD-054)', () => {
  it('waits for the socket of the link: while the shell says origin it tries again, five times in all, each with a new code', async () => {
    const waits: number[] = [];
    const wait = (milliseconds: number): Promise<void> => {
      waits.push(milliseconds);
      return Promise.resolve();
    };
    const late = { ...parts(), wait };
    late.install.mockResolvedValueOnce({ ok: false, problem: 'origin', message: 'x' }).mockResolvedValueOnce({ ok: false, problem: 'origin', message: 'x' });
    expect(await installDaemonHere(late)).toEqual({ kind: 'installed' });
    expect(late.enrol).toHaveBeenCalledTimes(3);
    expect(waits).toEqual([30_000, 30_000]);

    forgetDaemonHere();
    const never = { ...parts({ install: { ok: false, problem: 'origin', message: 'x' } }), wait };
    expect(await installDaemonHere(never)).toEqual({ kind: 'failed', step: 'origin' });
    expect(never.install).toHaveBeenCalledTimes(5);
  });
});
