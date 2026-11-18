// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { signInPathFor } from '$lib/access/routes';
import type { DeviceLinkStatus, Joined, LinkResult } from '$lib/desktop/link/shell';
import type { HubAddress, HubResult } from '$lib/hub/hub';
import { openAccess, sealAccess, type HubAccess } from './access';
import { joinHostedHub } from './join';
import { isHostedHub, markHosted } from './mark';

const INVITE = 'anima-invite.1.c2VjcmV0';
const CLAIM = 'one-time-claim';
const TOKEN = 'anima_token_secret';
const ADDRESS = 'http://127.0.0.1:41000';

const JOINED: LinkResult<Joined> = { ok: true, value: { fingerprint: 'ab12', port: 41_000 } };
const ON_LINK: LinkResult<DeviceLinkStatus> = { ok: true, value: { joined: true, state: 'up', port: 41_000 } };
const CLAIMED: HubResult<string> = { ok: true, value: TOKEN };

interface Answers {
  readonly join?: LinkResult<Joined>;
  readonly link?: LinkResult<DeviceLinkStatus>;
  readonly claim?: HubResult<string>;
  readonly write?: () => Promise<void>;
}

/** Parts that record each step in order; nothing reaches a shell, a key store or a hub. */
function parts(answers: Answers = {}) {
  const order: string[] = [];
  return {
    order,
    join: vi.fn((invite: string) => {
      order.push(`join ${invite}`);
      return Promise.resolve(answers.join ?? JOINED);
    }),
    link: vi.fn(() => {
      order.push('link');
      return Promise.resolve(answers.link ?? ON_LINK);
    }),
    claim: vi.fn((address: HubAddress, claim: string) => {
      order.push(`claim ${address.origin} ${claim}`);
      return Promise.resolve(answers.claim ?? CLAIMED);
    }),
    clearToken: vi.fn((address: HubAddress) => {
      order.push(`clear ${address.origin}`);
      return Promise.resolve();
    }),
    writeToken: vi.fn((address: HubAddress, token: string) => {
      order.push(`write ${address.origin} ${token}`);
      return answers.write?.() ?? Promise.resolve();
    }),
    saveAddress: vi.fn((origin: string) => {
      order.push(`save ${origin}`);
    }),
  };
}

const access = (invite: string | null = INVITE) => sealAccess({ invite, claim: CLAIM });

describe('the hub access (hosted-mode.md Section 4.3)', () => {
  it('LOG-007: is opaque: it shows and serialises no field, and only openAccess reads it', () => {
    const sealed = access();
    expect(JSON.stringify(sealed)).toBe('{"kind":"hub-access"}');
    expect(openAccess(sealed)).toEqual({ invite: INVITE, claim: CLAIM });
    expect(openAccess({ kind: 'hub-access' })).toBeNull();
  });
});

describe('joinHostedHub (R-129, HMD-037, S3)', () => {
  it('joins the link, removes the old token, trades the claim for a token, keeps it, then saves the address', async () => {
    const join = parts();
    expect(await joinHostedHub(access(), join)).toEqual({ ok: true, value: undefined });
    expect(join.order).toEqual([
      `join ${INVITE}`,
      `clear ${ADDRESS}`,
      `claim ${ADDRESS} ${CLAIM}`,
      `write ${ADDRESS} ${TOKEN}`,
      `save ${ADDRESS}`,
    ]);
  });

  it('a device already on the link gets its new token through that link, with no join', async () => {
    const join = parts();
    expect(await joinHostedHub(access(null), join)).toEqual({ ok: true, value: undefined });
    expect(join.order[0]).toBe('link');
    expect(join.join).not.toHaveBeenCalled();
    expect(join.saveAddress).toHaveBeenCalledWith(ADDRESS);
  });

  it('with no invite and no link this computer cannot reach the hub: refused', async () => {
    const join = parts({ link: { ok: true, value: { joined: false, state: 'stopped' } } });
    expect(await joinHostedHub(access(null), join)).toEqual({ ok: false, problem: 'refused' });
    expect(join.claim).not.toHaveBeenCalled();
  });

  it.each([
    ['no-answer', 'offline'],
    ['refused', 'refused'],
    ['bad-invite', 'refused'],
  ] as const)('a link join refused with %s is %s, and nothing is kept', async (problem, expected) => {
    const join = parts({ join: { ok: false, problem, message: 'x' } });
    expect(await joinHostedHub(access(), join)).toEqual({ ok: false, problem: expected });
    expect(join.claim).not.toHaveBeenCalled();
    expect(join.saveAddress).not.toHaveBeenCalled();
  });

  it.each([
    ['unreachable', 'offline'],
    ['UNAVAILABLE', 'offline'],
    ['UNAUTHENTICATED', 'refused'],
  ] as const)('a claim answered %s is %s, and no token or address is kept', async (code, expected) => {
    const join = parts({ claim: { ok: false, error: { code, message: 'x' } } });
    expect(await joinHostedHub(access(), join)).toEqual({ ok: false, problem: expected });
    expect(join.writeToken).not.toHaveBeenCalled();
    expect(join.saveAddress).not.toHaveBeenCalled();
  });

  it('S3: a key store that refuses the token saves no address', async () => {
    const join = parts({ write: () => Promise.reject(new Error('denied')) });
    expect(await joinHostedHub(access(), join)).toEqual({ ok: false, problem: 'refused' });
    expect(join.saveAddress).not.toHaveBeenCalled();
  });

  it('an access that the cloud did not seal is refused before the link', async () => {
    const join = parts();
    const forged: HubAccess = { kind: 'hub-access' };
    expect(await joinHostedHub(forged, join)).toEqual({ ok: false, problem: 'refused' });
    expect(join.join).not.toHaveBeenCalled();
  });
});

describe('the hosted mark (R-129)', () => {
  it('a hosted hub signs in again through the choices, never the password page', () => {
    expect(signInPathFor(true)).toBe('/welcome/hosted');
    expect(signInPathFor(false)).toBe('/sign-in');
  });

  it('marks only the address it was given', () => {
    markHosted(localStorage, ADDRESS);
    expect(isHostedHub(localStorage, ADDRESS)).toBe(true);
    expect(isHostedHub(localStorage, 'http://127.0.0.1:41001')).toBe(false);
    localStorage.clear();
  });
});
