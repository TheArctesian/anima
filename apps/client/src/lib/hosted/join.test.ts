import { describe, expect, it, vi } from 'vitest';
import type { Joined, LinkResult } from '$lib/desktop/link/shell';
import type { HubAddress } from '$lib/hub/hub';
import { openAccess, sealAccess, type HubAccess } from './access';
import { joinHostedHub } from './join';

const LINK = 'anima-invite.1.c2VjcmV0';
const TOKEN = 'anima_token_secret';

function parts(answer: LinkResult<Joined>, write: () => Promise<void> = () => Promise.resolve()) {
  const order: string[] = [];
  return {
    order,
    join: vi.fn((link: string) => {
      order.push(`join ${link}`);
      return Promise.resolve(answer);
    }),
    clearToken: vi.fn((address: HubAddress) => {
      order.push(`clear ${address.origin}`);
      return Promise.resolve();
    }),
    writeToken: vi.fn((address: HubAddress, token: string) => {
      order.push(`write ${address.origin} ${token}`);
      return write();
    }),
    saveAddress: vi.fn((origin: string) => {
      order.push(`save ${origin}`);
    }),
  };
}

describe('the hub access (hosted-mode.md Section 4.3)', () => {
  it('LOG-007: is opaque: it shows and serialises no field, and only openAccess reads it', () => {
    const access = sealAccess({ link: LINK, token: TOKEN });
    expect(JSON.stringify(access)).toBe('{"kind":"hub-access"}');
    expect(openAccess(access)).toEqual({ link: LINK, token: TOKEN });
    expect(openAccess({ kind: 'hub-access' })).toBeNull();
  });
});

describe('joinHostedHub (Section 4.3, S3)', () => {
  it('joins the link, puts the new token in place of the old one, and only then saves the address', async () => {
    const join = parts({ ok: true, value: { fingerprint: 'ab12', port: 41_000 } });
    expect(await joinHostedHub(sealAccess({ link: LINK, token: TOKEN }), join)).toEqual({ ok: true, value: undefined });
    expect(join.order).toEqual([
      `join ${LINK}`,
      'clear http://127.0.0.1:41000',
      `write http://127.0.0.1:41000 ${TOKEN}`,
      'save http://127.0.0.1:41000',
    ]);
  });

  it.each([
    ['no-answer', 'offline'],
    ['refused', 'refused'],
    ['bad-invite', 'refused'],
  ] as const)('a link join refused with %s is %s, and nothing is kept', async (problem, expected) => {
    const join = parts({ ok: false, problem, message: 'x' });
    expect(await joinHostedHub(sealAccess({ link: LINK, token: TOKEN }), join)).toEqual({ ok: false, problem: expected });
    expect(join.writeToken).not.toHaveBeenCalled();
    expect(join.saveAddress).not.toHaveBeenCalled();
  });

  it('S3: a key store that refuses the token saves no address', async () => {
    const join = parts({ ok: true, value: { fingerprint: 'ab12', port: 41_000 } }, () => Promise.reject(new Error('denied')));
    expect(await joinHostedHub(sealAccess({ link: LINK, token: TOKEN }), join)).toEqual({ ok: false, problem: 'refused' });
    expect(join.saveAddress).not.toHaveBeenCalled();
  });

  it('an access that the cloud did not seal is refused before the link', async () => {
    const join = parts({ ok: true, value: { fingerprint: 'ab12', port: 41_000 } });
    const forged: HubAccess = { kind: 'hub-access' };
    expect(await joinHostedHub(forged, join)).toEqual({ ok: false, problem: 'refused' });
    expect(join.join).not.toHaveBeenCalled();
  });
});
