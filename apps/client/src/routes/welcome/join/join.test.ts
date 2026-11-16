import { describe, expect, it, vi } from 'vitest';
import type { LinkResult, Joined } from '$lib/desktop/link/shell';
import { deviceLinkAddress, joinHub } from './join';

const INVITE = 'anima-invite.1.c2VjcmV0';

function parts(answer: LinkResult<Joined>, clear: () => Promise<void> = () => Promise.resolve()) {
  const order: string[] = [];
  return {
    order,
    join: vi.fn(() => {
      order.push('join');
      return Promise.resolve(answer);
    }),
    clearToken: vi.fn(() => {
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
    expect(await joinHub(`  ${INVITE}\n`, join)).toEqual({ ok: true, fingerprint: 'ab12-cd34', origin: 'http://127.0.0.1:41000' });
    expect(join.join).toHaveBeenCalledWith(INVITE);
    expect(join.order).toEqual(['join', 'clear', 'save']);
    expect(join.clearToken.mock.calls[0]?.[0]).toMatchObject({ origin: 'http://127.0.0.1:41000' });
    expect(join.saveAddress).toHaveBeenCalledWith('http://127.0.0.1:41000');
  });

  it('S3: when the old token cannot be removed the address is not saved', async () => {
    const join = parts({ ok: true, value: { fingerprint: 'f', port: 41_000 } }, () => Promise.reject(new Error('denied')));
    const outcome = await joinHub(INVITE, join);
    expect(outcome.ok).toBe(false);
    expect(join.saveAddress).not.toHaveBeenCalled();
  });

  it('LNK-003, LOG-007: a refused join shows the fixed message and never the invite', async () => {
    const message = 'The hub did not answer: the invite is used, expired or wrong, or the hub is off.';
    const join = parts({ ok: false, problem: 'no-answer', message });
    const outcome = await joinHub(INVITE, join);
    expect(outcome).toEqual({ ok: false, message });
    expect(join.clearToken).not.toHaveBeenCalled();
    expect(JSON.stringify(outcome)).not.toContain(INVITE);
  });

  it('FRN-005: an empty paste sends nothing', async () => {
    const join = parts({ ok: true, value: { fingerprint: 'f', port: 1 } });
    expect(await joinHub('   ', join)).toEqual({ ok: false, message: 'Paste the invite from the hub.' });
    expect(join.join).not.toHaveBeenCalled();
  });
});
