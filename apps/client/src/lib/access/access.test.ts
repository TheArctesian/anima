import { describe, expect, it, vi } from 'vitest';
import { AccessTokenError } from '$lib/desktop/token';
import { DEFAULT_HUB_ADDRESS } from '$lib/hub/hub';
import { accessApi } from './api';
import { fakeHub, failure, ok } from './access.fixture';
import { signInNeeded, startGuard } from './guard';
import { signOut, TOKEN_KEPT_MESSAGE } from './sign-out';

const ADDRESS = DEFAULT_HUB_ADDRESS;

describe('accessApi', () => {
  it('OD-2: login sends the user name, password and totpCode and returns the token', async () => {
    const { hub, sent } = fakeHub({ Login: ok({ login: { token: 'tok-1' } }) });
    const result = await accessApi(hub).login({ username: 'ann', password: 'pw', totpCode: '123456' });
    expect(result).toEqual({ ok: true, value: 'tok-1' });
    expect(sent).toEqual([{ operation: 'Login', variables: { input: { username: 'ann', password: 'pw', totpCode: '123456' } } }]);
  });

  it('SET-002: me returns the user name and logout returns the hub answer', async () => {
    const { hub } = fakeHub({ Me: ok({ me: { username: 'ann' } }), Logout: ok({ logout: true }) });
    const api = accessApi(hub);
    expect(await api.me()).toEqual({ ok: true, value: { username: 'ann' } });
    expect(await api.logout()).toEqual({ ok: true, value: true });
  });

  it('API-003: a failed call keeps its typed error', async () => {
    const { hub } = fakeHub({ Me: failure('UNAUTHENTICATED') });
    const result = await accessApi(hub).me();
    expect(!result.ok && result.error.code).toBe('UNAUTHENTICATED');
  });
});

describe('startGuard', () => {
  function check(pathname: string, token: string | null, desktop = true) {
    const goToSignIn = vi.fn();
    const read = vi.fn(() => Promise.resolve(token));
    return { goToSignIn, read, run: () => startGuard({ pathname, address: ADDRESS, token: read, desktop, goToSignIn }) };
  }

  it('API-003: with no stored token the guard goes to sign-in', async () => {
    const c = check('/devices', null);
    await c.run();
    expect(c.goToSignIn).toHaveBeenCalledOnce();
    expect(c.read).toHaveBeenCalledWith(ADDRESS);
  });

  it('API-003: an unreadable key store also goes to sign-in', async () => {
    const goToSignIn = vi.fn();
    const token = () => Promise.reject(new AccessTokenError('failed'));
    await startGuard({ pathname: '/devices', address: ADDRESS, token, desktop: true, goToSignIn });
    expect(goToSignIn).toHaveBeenCalledOnce();
  });

  it('API-003: with a stored token the guard does nothing', async () => {
    const c = check('/devices', 'tok');
    await c.run();
    expect(c.goToSignIn).not.toHaveBeenCalled();
  });

  it('H-5: on /sign-in the guard does nothing and does not read the token', async () => {
    const c = check('/sign-in', null);
    await c.run();
    expect(c.goToSignIn).not.toHaveBeenCalled();
    expect(c.read).not.toHaveBeenCalled();
  });

  it('D-4: outside the desktop app the guard does nothing', async () => {
    const c = check('/devices', null, false);
    await c.run();
    expect(c.goToSignIn).not.toHaveBeenCalled();
  });
});

describe('signInNeeded', () => {
  it('API-003: UNAUTHENTICATED goes to sign-in', () => {
    const go = vi.fn();
    expect(signInNeeded({ code: 'UNAUTHENTICATED', message: 'x' }, go)).toBe(true);
    expect(go).toHaveBeenCalledOnce();
  });

  it('API-003: other errors stay where they are', () => {
    const go = vi.fn();
    expect(signInNeeded({ code: 'unreachable', message: 'x' }, go)).toBe(false);
    expect(go).not.toHaveBeenCalled();
  });
});

describe('signOut', () => {
  it('CLI-003: logout runs first, then the token is cleared with hub.address', async () => {
    const order: string[] = [];
    const api = {
      logout: () => {
        order.push('logout');
        return Promise.resolve(ok(true));
      },
    };
    const clearToken = vi.fn(() => {
      order.push('clear');
      return Promise.resolve();
    });
    expect(await signOut({ api, address: ADDRESS, clearToken })).toBeNull();
    expect(order).toEqual(['logout', 'clear']);
    expect(clearToken).toHaveBeenCalledWith(ADDRESS);
  });

  it('CLI-003: the token is cleared even when logout fails', async () => {
    const { hub } = fakeHub({ Logout: failure('unreachable') });
    const clearToken = vi.fn(() => Promise.resolve());
    expect(await signOut({ api: accessApi(hub), address: ADDRESS, clearToken })).toBeNull();
    expect(clearToken).toHaveBeenCalledWith(ADDRESS);
  });

  it('CLI-003: a key store that cannot clear is reported, with fixed text', async () => {
    const { hub } = fakeHub({ Logout: ok({ logout: true }) });
    const clearToken = () => Promise.reject(new AccessTokenError('denied'));
    expect(await signOut({ api: accessApi(hub), address: ADDRESS, clearToken })).toBe(TOKEN_KEPT_MESSAGE);
  });
});
