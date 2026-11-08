import { describe, expect, it, vi } from 'vitest';
import { AccessTokenError } from '$lib/desktop/token';
import type { HubResult } from '$lib/hub/hub';
import { accessApi } from './api';
import { fakeHub, failure, HUB_ADDRESS, ok } from './access.fixture';
import { isBare, isSignedOut, routePath, signInNeeded, startGuard } from './guard';
import { signOut, TOKEN_KEPT_AFTER_LOGOUT, TOKEN_KEPT_UNCONFIRMED } from './sign-out';

const ADDRESS = HUB_ADDRESS;

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

describe('startGuard on desktop', () => {
  function check(pathname: string, token: string | null, desktop = true) {
    const goToSignIn = vi.fn();
    const read = vi.fn(() => Promise.resolve(token));
    const me = vi.fn(() => Promise.resolve(ok({ username: 'ann' })));
    return { goToSignIn, read, me, run: () => startGuard({ pathname, address: ADDRESS, token: read, desktop, me, goToSignIn }) };
  }

  it('API-003: the guard reads the token for the hub address it is given', async () => {
    const c = check('/devices', null);
    await c.run();
    expect(c.read).toHaveBeenCalledWith(ADDRESS);
  });

  it('API-003: an unreadable key store also goes to sign-in', async () => {
    const goToSignIn = vi.fn();
    const token = () => Promise.reject(new AccessTokenError('failed'));
    const me = vi.fn();
    await startGuard({ pathname: '/devices', address: ADDRESS, token, desktop: true, me, goToSignIn });
    expect(goToSignIn).toHaveBeenCalledOnce();
  });

  it.each([null, ''])('API-003: with the stored token %j the guard goes to sign-in', async (stored) => {
    const c = check('/devices', stored);
    await c.run();
    expect(c.goToSignIn).toHaveBeenCalledOnce();
  });

  it('API-003: with a stored token the guard does nothing and the desktop app does not ask the hub', async () => {
    const c = check('/devices', 'tok');
    await c.run();
    expect(c.goToSignIn).not.toHaveBeenCalled();
    expect(c.me).not.toHaveBeenCalled();
  });

  it('H-5: on /sign-in the guard does nothing and does not read the token', async () => {
    const c = check('/sign-in', null);
    await c.run();
    expect(c.goToSignIn).not.toHaveBeenCalled();
    expect(c.read).not.toHaveBeenCalled();
  });
});

describe('startGuard in a browser', () => {
  /** A browser start: no key store; the hub answers `me` with `answer`. */
  function browser(pathname: string, answer: HubResult<unknown>) {
    const goToSignIn = vi.fn();
    const token = vi.fn(() => Promise.resolve(null));
    const me = vi.fn(() => Promise.resolve(answer));
    return { goToSignIn, me, token, run: () => startGuard({ pathname, address: ADDRESS, token, desktop: false, me, goToSignIn }) };
  }

  it('API-003: in a browser, UNAUTHENTICATED from me goes to sign-in', async () => {
    const c = browser('/devices', failure('UNAUTHENTICATED'));
    await c.run();
    expect(c.me).toHaveBeenCalledOnce();
    expect(c.goToSignIn).toHaveBeenCalledOnce();
    expect(c.token).not.toHaveBeenCalled();
  });

  it.each(['unreachable', 'INTERNAL'] as const)('API-003: in a browser, %s from me stays where it is', async (code) => {
    const c = browser('/devices/d1', failure(code));
    await c.run();
    expect(c.me).toHaveBeenCalledOnce();
    expect(c.goToSignIn).not.toHaveBeenCalled();
  });

  it('API-003: in a browser, a signed-in me stays where it is', async () => {
    const c = browser('/', ok({ me: { username: 'ann' } }));
    await c.run();
    expect(c.goToSignIn).not.toHaveBeenCalled();
  });

  it.each(['/sign-in', '/shared', '/shared/', '/shared/abc'])('E-3: on the signed-out route %s the browser guard never asks the hub', async (path) => {
    const c = browser(path, failure('UNAUTHENTICATED'));
    await c.run();
    expect(c.me).not.toHaveBeenCalled();
    expect(c.goToSignIn).not.toHaveBeenCalled();
  });
});

describe('signed-out routes', () => {
  it.each(['/sign-in', '/sign-in/', '/shared', '/shared/', '/shared/abc', '/shared/abc/def'])('E-3: %s needs no sign-in', (path) => {
    expect(isSignedOut(path)).toBe(true);
  });

  it.each(['/sharedx', '/sign-in-x', '/sign-inx', '/devices', '/', '/settings', '/Shared', '/devices/shared', '/shared%2Fx'])(
    'E-3: %s needs sign-in',
    (path) => {
      expect(isSignedOut(path)).toBe(false);
    },
  );

  it('SEC-002: only /shared and below render bare; /sign-in keeps the shell for its mutation', () => {
    expect(['/shared', '/shared/', '/shared/abc'].map(isBare)).toEqual([true, true, true]);
    expect(['/sign-in', '/sharedx', '/devices', '/'].map(isBare)).toEqual([false, false, false, false]);
  });

  it('E-3: the route path is the one the router matches: below the base path, percent-decoded except %25', () => {
    expect(routePath('/shared/abc', '')).toBe('/shared/abc');
    expect(routePath('/%73hared', '')).toBe('/shared');
    expect(routePath('/anima/shared', '/anima')).toBe('/shared');
    expect(routePath('/anima', '/anima')).toBe('/');
    expect(routePath('/animax/shared', '/anima')).toBe('/animax/shared');
    expect(routePath('/shared%2Fx', '')).toBe('/shared%2Fx');
    expect(routePath('/shared%25x', '')).toBe('/shared%25x');
  });

  it('E-3: a path the router cannot decode stays as it is', () => {
    expect(routePath('/shared/%E0%A4%A', '')).toBe('/shared/%E0%A4%A');
    expect(routePath('/devices%E0', '')).toBe('/devices%E0');
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

  it('CLI-003: the token is cleared even when logout rejects', async () => {
    const api = { logout: () => Promise.reject(new Error('boom')) };
    const clearToken = vi.fn(() => Promise.resolve());
    expect(await signOut({ api, address: ADDRESS, clearToken })).toBeNull();
    expect(clearToken).toHaveBeenCalledWith(ADDRESS);
  });

  it('SET-002: a key store that cannot clear after a good logout says the hub signed out', async () => {
    const { hub } = fakeHub({ Logout: ok({ logout: true }) });
    const clearToken = () => Promise.reject(new AccessTokenError('denied'));
    expect(await signOut({ api: accessApi(hub), address: ADDRESS, clearToken })).toBe(TOKEN_KEPT_AFTER_LOGOUT);
  });

  it('SET-002: a key store that cannot clear after a failed logout does not claim the hub signed out', async () => {
    const { hub } = fakeHub({ Logout: failure('unreachable') });
    const clearToken = () => Promise.reject(new AccessTokenError('denied'));
    expect(await signOut({ api: accessApi(hub), address: ADDRESS, clearToken })).toBe(TOKEN_KEPT_UNCONFIRMED);
  });

  it('CLI-003: a plain Error from the key store gives the token-kept text', async () => {
    const { hub } = fakeHub({ Logout: ok({ logout: true }) });
    const clearToken = () => Promise.reject(new Error('anything'));
    expect(await signOut({ api: accessApi(hub), address: ADDRESS, clearToken })).toBe(TOKEN_KEPT_AFTER_LOGOUT);
  });

  it('CLI-003: without a key store (not-desktop) there is nothing to clear and sign-out completes', async () => {
    const { hub } = fakeHub({ Logout: failure('UNAUTHENTICATED') });
    const clearToken = () => Promise.reject(new AccessTokenError('not-desktop'));
    expect(await signOut({ api: accessApi(hub), address: ADDRESS, clearToken })).toBeNull();
  });
});
