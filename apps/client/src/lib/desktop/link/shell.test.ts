import { beforeEach, describe, expect, it, vi } from 'vitest';
import { joinedOf, joinLink, leaveLink, linkStatusOf, readLink } from './shell';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const INVITE = 'anima-invite.1.c2VjcmV0LWludml0ZQ';

beforeEach(() => {
  shell.invoke.mockReset().mockResolvedValue(null);
  shell.isTauri.mockReturnValue(true);
});

describe('the device link answers', () => {
  it('FRN-005, F-2: a join gives the fingerprint and a real port', () => {
    expect(joinedOf({ fingerprint: 'ab12-cd34', port: 41_000 })).toEqual({ fingerprint: 'ab12-cd34', port: 41_000 });
    for (const wrong of [null, {}, { fingerprint: '', port: 1 }, { fingerprint: 'x', port: 0 }, { fingerprint: 'x', port: 70_000 }, { fingerprint: 'x', port: '80' }]) {
      expect(joinedOf(wrong)).toBeUndefined();
    }
  });

  it('LNK-006: the status keeps the path and latency it was sent, and refuses unknown values', () => {
    const up = { joined: true, state: 'up', port: 41_000, path: 'relayed', latencyMs: 42.5 };
    expect(linkStatusOf(up)).toEqual(up);
    expect(linkStatusOf({ joined: true, state: 'down', reason: 'port-taken', path: null })).toEqual({ joined: true, state: 'down', reason: 'port-taken' });
    for (const wrong of [{ ...up, state: 'sideways' }, { ...up, path: 'tunnel' }, { ...up, reason: 'other' }, { state: 'up' }]) expect(linkStatusOf(wrong)).toBeUndefined();
  });
});

describe('the shell calls', () => {
  it('FRN-005: join passes only the invite; status and leave pass nothing', async () => {
    shell.invoke.mockImplementation((command: string) =>
      Promise.resolve(command === 'link_join' ? { fingerprint: 'f', port: 41_000 } : command === 'link_status' ? { joined: false, state: 'stopped' } : null),
    );
    expect(await joinLink(INVITE)).toEqual({ ok: true, value: { fingerprint: 'f', port: 41_000 } });
    expect(await readLink()).toEqual({ ok: true, value: { joined: false, state: 'stopped' } });
    expect(await leaveLink()).toEqual({ ok: true, value: null });
    expect(shell.invoke.mock.calls).toEqual([['link_join', { invite: INVITE }], ['link_status', undefined], ['link_leave', undefined]]);
  });

  it('LOG-007, LNK-003: a refused join keeps the code and a fixed message, never the invite or the shell text', async () => {
    shell.invoke.mockRejectedValue({ code: 'no-answer', message: `raw ${INVITE}` });
    const result = await joinLink(INVITE);
    expect(result).toEqual({ ok: false, problem: 'no-answer', message: 'The hub did not answer: the invite is used, expired or wrong, or the hub is off.' });
    expect(JSON.stringify(result)).not.toContain(INVITE);
    shell.invoke.mockRejectedValue(new Error('boom'));
    expect(await joinLink(INVITE)).toMatchObject({ ok: false, problem: 'failed' });
  });

  it('CLI-001: outside the desktop app nothing is sent', async () => {
    shell.isTauri.mockReturnValue(false);
    expect(await joinLink(INVITE)).toMatchObject({ ok: false, problem: 'unavailable' });
    expect(shell.invoke).not.toHaveBeenCalled();
  });
});
