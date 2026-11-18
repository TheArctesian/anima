import { beforeEach, describe, expect, it, vi } from 'vitest';
import { openAccess } from '$lib/hosted/access';
import { cloudAccount } from './shell';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const CODE = { code: 'BCDF-GHJK', page: 'https://auth.example/device?user_code=BCDF-GHJK', interval: 5 };

/** A shell that answers each command from `answers`; a function answer may throw a shell error. */
function answering(answers: Record<string, unknown>) {
  shell.invoke.mockImplementation((name: string) => {
    const answer = answers[name];
    return typeof answer === 'function' ? Promise.resolve().then(answer as () => unknown) : Promise.resolve(answer);
  });
}

const fails = (code: string) => () => {
  throw { code };
};

beforeEach(() => {
  shell.invoke.mockReset();
  shell.isTauri.mockReturnValue(true);
});

describe('the real cloud account (HMD-043)', () => {
  it('a session that still holds signs in again with no browser step (item 6)', async () => {
    answering({ cloud_sign_in_resume: true });
    expect(await cloudAccount().start()).toEqual({ ok: true, value: 'signed-in' });
    expect(shell.invoke).not.toHaveBeenCalledWith('cloud_sign_in_start', undefined);
  });

  it('with no session it starts the device sign-in and shows only the code and the page', async () => {
    answering({ cloud_sign_in_resume: false, cloud_sign_in_start: CODE });
    expect(await cloudAccount().start()).toEqual({ ok: true, value: CODE });
  });

  it('a build with no client ID says so at once', async () => {
    answering({ cloud_sign_in_resume: fails('unconfigured') });
    expect(await cloudAccount().start()).toEqual({ ok: false, problem: 'unconfigured' });
  });

  it('polls: false is waiting, true is signed in', async () => {
    answering({ cloud_sign_in_poll: false });
    expect(await cloudAccount().poll(CODE)).toEqual({ ok: true, value: 'waiting' });
    answering({ cloud_sign_in_poll: true });
    expect(await cloudAccount().poll(CODE)).toEqual({ ok: true, value: 'signed-in' });
  });

  it.each([
    ['cancelled', 'cancelled'],
    ['expired', 'expired'],
    ['offline', 'offline'],
    ['not-ready', 'offline'],
    ['payment-failed', 'payment'],
    ['unauthenticated', 'signed-out'],
    ['refused', 'refused'],
    ['caller', 'refused'],
  ])('a shell error %s is the problem %s', async (code, problem) => {
    answering({ cloud_hub: fails(code) });
    expect(await cloudAccount().hub()).toEqual({ ok: false, problem });
  });

  it('the hub state is checked: anything else is a refusal', async () => {
    answering({ cloud_hub: 'waking' });
    expect(await cloudAccount().hub()).toEqual({ ok: true, value: 'waking' });
    answering({ cloud_hub: 'sideways' });
    expect(await cloudAccount().hub()).toEqual({ ok: false, problem: 'refused' });
  });

  it('LOG-007: the invite and the claim come back sealed; a device on the link gets no invite', async () => {
    answering({ cloud_hub_access: { invite: 'anima-invite.1.x', claim: 'secret' } });
    const access = await cloudAccount().access();
    if (!access.ok) throw new Error('refused');
    expect(JSON.stringify(access.value)).toBe('{"kind":"hub-access"}');
    expect(openAccess(access.value)).toEqual({ invite: 'anima-invite.1.x', claim: 'secret' });
    answering({ cloud_hub_access: { invite: null, claim: 'secret' } });
    const again = await cloudAccount().access();
    expect(again.ok && openAccess(again.value)).toEqual({ invite: null, claim: 'secret' });
    answering({ cloud_hub_access: { invite: 'x', token: 'secret' } });
    expect(await cloudAccount().access()).toEqual({ ok: false, problem: 'refused' });
  });

  it('outside the desktop shell there is no sign-in', async () => {
    shell.isTauri.mockReturnValue(false);
    expect(await cloudAccount().start()).toEqual({ ok: false, problem: 'unconfigured' });
    expect(shell.invoke).not.toHaveBeenCalled();
  });
});
