import { describe, expect, it, vi } from 'vitest';
import type { LinkResult, Joined } from '$lib/desktop/link/shell';
import type { HubAddress } from '$lib/hub/hub';
import { deviceLinkAddress } from '$lib/link/devices';
import { finishJoin, joinHub } from './join';

const INVITE = 'anima-invite.1.c2VjcmV0';

function parts(answer: LinkResult<Joined>, clear: () => Promise<void> = () => Promise.resolve()) {
  const order: string[] = [];
  return {
    order,
    join: vi.fn(() => {
      order.push('join');
      return Promise.resolve(answer);
    }),
    clearToken: vi.fn<(address: HubAddress) => Promise<void>>(() => {
      order.push('clear');
      return clear();
    }),
    saveAddress: vi.fn(() => {
      order.push('save');
    }),
  };
}

describe('joinHub (FRN-005)', () => {
  it('F-1, F-2: the device link address is 127.0.0.1 and the port', () => {
    expect(deviceLinkAddress(41_000)?.origin).toBe('http://127.0.0.1:41000');
  });

  it('FRN-005, S3: on success the old token of the address goes first, then the address is saved', async () => {
    const join = parts({ ok: true, value: { fingerprint: 'ab12-cd34', port: 41_000 } });
    expect(await joinHub(`  ${INVITE}\n`, join)).toEqual({
      joined: true,
      device: { fingerprint: 'ab12-cd34', port: 41_000 },
      finished: { ok: true, origin: 'http://127.0.0.1:41000' },
    });
    expect(join.join).toHaveBeenCalledWith(INVITE);
    expect(join.order).toEqual(['join', 'clear', 'save']);
    expect(join.clearToken.mock.calls[0]?.[0]).toMatchObject({ origin: 'http://127.0.0.1:41000' });
    expect(join.saveAddress).toHaveBeenCalledWith('http://127.0.0.1:41000');
  });

  it('S3: when the old token cannot be removed the device is still joined, the address is not saved, and a retry finishes', async () => {
    const clear = vi.fn<() => Promise<void>>().mockRejectedValueOnce(new Error('denied')).mockResolvedValue(undefined);
    const join = parts({ ok: true, value: { fingerprint: 'f', port: 41_000 } }, clear);
    const outcome = await joinHub(INVITE, join);
    expect(outcome).toMatchObject({ joined: true, device: { fingerprint: 'f' }, finished: { ok: false } });
    expect(join.saveAddress).not.toHaveBeenCalled();
    expect(await finishJoin({ fingerprint: 'f', port: 41_000 }, join)).toEqual({ ok: true, origin: 'http://127.0.0.1:41000' });
    expect(join.join).toHaveBeenCalledOnce();
    expect(join.saveAddress).toHaveBeenCalledWith('http://127.0.0.1:41000');
  });

  it('LNK-003, LOG-007: a refused join shows the fixed message and never the invite; the hub may have used it', async () => {
    const message = 'The hub did not answer: the invite is used, expired or wrong, or the hub is off.';
    const join = parts({ ok: false, problem: 'no-answer', message });
    const outcome = await joinHub(INVITE, join);
    expect(outcome).toEqual({ joined: false, message, inviteKept: false });
    expect(join.clearToken).not.toHaveBeenCalled();
    expect(JSON.stringify(outcome)).not.toContain(INVITE);
  });

  it.each(['busy', 'bad-invite', 'not-carried'] as const)('FRN-005: a join refused before the hub saw the invite (%s) keeps it usable', async (problem) => {
    const outcome = await joinHub(INVITE, parts({ ok: false, problem, message: 'x' }));
    expect(outcome).toEqual({ joined: false, message: 'x', inviteKept: true });
  });

  it('FRN-005: an empty paste sends nothing', async () => {
    const join = parts({ ok: true, value: { fingerprint: 'f', port: 1 } });
    expect(await joinHub('   ', join)).toEqual({ joined: false, message: 'Paste the invite from the hub.', inviteKept: true });
    expect(join.join).not.toHaveBeenCalled();
  });
});
