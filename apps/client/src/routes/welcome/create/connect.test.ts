import { describe, expect, it, vi } from 'vitest';
import { fakeHub, failure, ok } from '$lib/access/access.fixture';
import { accessApi } from '$lib/access/api';
import type { Listener } from '$lib/desktop/hub-listener/shell';
import type { LocalHubStatus } from '$lib/desktop/local-hub/shell';
import { POLL_MS } from '$lib/local-hub/state';
import { connectLocalHub, localHubAddress, READY_READS, signInFirst } from './connect';

const READY: LocalHubStatus = { installed: true, running: true, state: 'ready', hub: '127.0.0.1:8443' };

function parts(status: LocalHubStatus, listener: Listener) {
  return {
    read: vi.fn(() => Promise.resolve({ ok: true as const, value: status })),
    check: vi.fn(() => Promise.resolve({ ok: true as const, listener })),
    saveAddress: vi.fn(),
    wait: vi.fn(() => Promise.resolve()),
  };
}

describe('connecting to the hub on this computer', () => {
  it('F-1: the address is http://127.0.0.1:<port> from a ready hub only', () => {
    expect(localHubAddress(READY)?.origin).toBe('http://127.0.0.1:8443');
    expect(localHubAddress({ ...READY, hub: 'localhost:8443' })).toBeNull();
    expect(localHubAddress({ ...READY, hub: '10.0.0.5:8443' })).toBeNull();
    expect(localHubAddress({ ...READY, hub: '[::1]:8443' })).toBeNull();
    expect(localHubAddress({ ...READY, state: 'starting' })).toBeNull();
  });

  it('F-10: the address is checked, then saved, only when a program of this user listens', async () => {
    const own = parts(READY, 'own');
    expect(await connectLocalHub(own)).toMatchObject({ ok: true, address: { origin: 'http://127.0.0.1:8443' } });
    expect(own.check).toHaveBeenCalledWith('http://127.0.0.1:8443');
    expect(own.saveAddress).toHaveBeenCalledWith('http://127.0.0.1:8443');
  });

  it.each(['other', 'none'] as const)('F-10: with %s listening nothing is saved and the screen says why', async (listener) => {
    const refused = parts(READY, listener);
    const connected = await connectLocalHub(refused);
    expect(connected.ok).toBe(false);
    expect(refused.saveAddress).not.toHaveBeenCalled();
  });

  it('FRN-002: a hub that stays not ready is read once a second for a while, then not checked', async () => {
    const starting = parts({ ...READY, state: 'starting' }, 'own');
    expect(await connectLocalHub(starting)).toEqual({ ok: false, message: 'The hub on this computer is not ready. Retry shortly.' });
    expect(starting.read).toHaveBeenCalledTimes(READY_READS);
    expect(starting.wait.mock.calls).toEqual(Array.from({ length: READY_READS - 1 }, () => [POLL_MS]));
    expect(starting.check).not.toHaveBeenCalled();
  });

  it('FRN-002: a hub that becomes ready while the step waits is checked and saved', async () => {
    const later = parts(READY, 'own');
    later.read.mockResolvedValueOnce({ ok: true, value: { ...READY, state: 'starting' } });
    expect(await connectLocalHub(later)).toMatchObject({ ok: true });
    expect(later.read).toHaveBeenCalledTimes(2);
    expect(later.saveAddress).toHaveBeenCalledWith('http://127.0.0.1:8443');
  });
});

describe('the first sign-in (FRN-004)', () => {
  const FORM = { name: 'ann', password: 'correct horse', code: '654321' };

  it('FRN-004, CLI-003: logs in with the name, the password and the next code, and keeps the token', async () => {
    const { hub, sent } = fakeHub({ Login: ok({ login: { token: 'tok-new' } }) });
    const keep = vi.fn(() => Promise.resolve());
    expect(await signInFirst(accessApi(hub), keep, FORM)).toEqual({ ok: true });
    expect(sent).toEqual([{ operation: 'Login', variables: { input: { username: 'ann', password: 'correct horse', totpCode: '654321' } } }]);
    expect(keep).toHaveBeenCalledWith('tok-new');
  });

  it('FRN-004: a refused code says to wait for the next one; nothing is kept', async () => {
    const { hub } = fakeHub({ Login: failure('UNAUTHENTICATED', 'hub text') });
    const keep = vi.fn();
    const outcome = await signInFirst(accessApi(hub), keep, FORM);
    expect(outcome).toEqual({ ok: false, message: 'The hub refused the code. Wait for the next one.' });
    expect(keep).not.toHaveBeenCalled();
  });

  it('CLI-003, LOG-007: a key store that refuses the token says so, with no token in the text', async () => {
    const { hub } = fakeHub({ Login: ok({ login: { token: 'tok-new' } }) });
    const outcome = await signInFirst(accessApi(hub), () => Promise.reject(new Error('tok-new')), FORM);
    expect(outcome.ok).toBe(false);
    expect(JSON.stringify(outcome)).not.toContain('tok-new');
  });
});
