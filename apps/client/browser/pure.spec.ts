import { expect, test } from '@playwright/test';
import { BEARER_TOKEN, browserSessions, callerOf, clearedCookie, HUB_ORIGIN, sessionCookie } from './synthetic/access';
import { isClosedError, noteClosing, untilClosed } from './synthetic/closed';
import { ticketBook } from './synthetic/tickets';

// The pure parts of the K7 and K4 model, in Node with no browser: a mutation here must fail a test, not pass unseen.

const NOW = new Date('2026-11-07T10:00:00Z');
const later = (seconds: number): Date => new Date(NOW.getTime() + seconds * 1000);

test.describe('callerOf (K7 "Authentication transport")', () => {
  const issued = () => {
    const sessions = browserSessions();
    return { sessions, token: sessions.issue() };
  };

  test('API-002: the cookie signs a request in only from the hub Origin', () => {
    const { sessions, token } = issued();
    expect(callerOf({ origin: HUB_ORIGIN, authorization: null, cookie: token }, sessions)).toBe('cookie');
    for (const origin of ['https://example.net', 'null', null]) {
      expect(callerOf({ origin, authorization: null, cookie: token }, sessions)).toBe('signed-out');
    }
  });

  test('API-002: a cookie the hub did not issue signs nobody in', () => {
    const { sessions } = issued();
    expect(callerOf({ origin: HUB_ORIGIN, authorization: null, cookie: 'made-up' }, sessions)).toBe('signed-out');
  });

  test('API-002: an Authorization header wins; a bad one is signed out even with a good cookie', () => {
    const { sessions, token } = issued();
    expect(callerOf({ origin: HUB_ORIGIN, authorization: `Bearer ${BEARER_TOKEN}`, cookie: token }, sessions)).toBe('bearer');
    expect(callerOf({ origin: HUB_ORIGIN, authorization: 'Bearer wrong', cookie: token }, sessions)).toBe('signed-out');
  });
});

test.describe('the Set-Cookie text (K7 "Authentication transport")', () => {
  test('SEC-002, D-3: the cookie is __Host-anima with Secure, HttpOnly, SameSite=Strict, Path=/, no Domain and seven days', () => {
    expect(sessionCookie('token-value')).toBe('__Host-anima=token-value; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=604800');
  });

  test('SEC-002, D-3: the clearing cookie has the same name and attributes with an empty value and Max-Age=0', () => {
    expect(clearedCookie()).toBe('__Host-anima=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0');
  });
});

test.describe('ticketBook (K4 section 1)', () => {
  test('SEC-002: a ticket is good once, on its own path', () => {
    const book = ticketBook();
    const ticket = book.issue('s1', '/terminal', NOW);
    expect(book.redeem(ticket.token, '/tunnel', NOW)).toEqual({ ok: false });
    // The wrong-path try spent it, as the hub does with any presented token.
    expect(book.redeem(ticket.token, '/terminal', NOW)).toEqual({ ok: false });
    const second = book.issue('s2', '/terminal', NOW);
    expect(book.redeem(second.token, '/terminal', NOW)).toEqual({ ok: true, sessionId: 's2' });
    expect(book.redeem(second.token, '/terminal', NOW)).toEqual({ ok: false });
  });

  test('SEC-002: a ticket lives 30 seconds', () => {
    const book = ticketBook();
    const fresh = book.issue('s1', '/tunnel', NOW);
    expect(book.redeem(fresh.token, '/tunnel', later(29))).toEqual({ ok: true, sessionId: 's1' });
    const stale = book.issue('s2', '/tunnel', NOW);
    expect(book.redeem(stale.token, '/tunnel', later(30))).toEqual({ ok: false });
  });

  test('SEC-002: no token, or an unknown one, is refused', () => {
    const book = ticketBook();
    expect(book.redeem(null, '/tunnel', NOW)).toEqual({ ok: false });
    expect(book.redeem('unknown', '/tunnel', NOW)).toEqual({ ok: false });
  });
});

test.describe('untilClosed (the end of a test is not a hub fault)', () => {
  test('MV-001: the protocol messages of a closed page, context or browser are closed errors, anything else is not', () => {
    for (const text of [
      'Target page, context or browser has been closed',
      'Could not find browser context',
      'Failed to find browser context',
      'browserContext.cookies: Browser has been closed',
    ]) {
      expect(isClosedError(new Error(text)), text).toBe(true);
    }
    const named = new Error('x');
    named.name = 'TargetClosedError';
    expect(isClosedError(named)).toBe(true);
    for (const other of [new Error('schema has no field me'), new TypeError('x is undefined'), 'has been closed', null]) {
      expect(isClosedError(other)).toBe(false);
    }
  });

  test('MV-001: the error a real closed context raises is classified as closed, so the hub drops it', async ({ browser }) => {
    const context = await browser.newContext();
    await context.close();
    const raised: unknown = await context.cookies().catch((error: unknown) => error);
    expect(raised).toBeInstanceOf(Error);
    expect(isClosedError(raised)).toBe(true);
    await untilClosed(async () => {
      await context.cookies();
    });
  });

  test('MV-001: any other failure is thrown on, even while a context is closed', async ({ browser }) => {
    const context = await browser.newContext();
    await context.close();
    await expect(
      untilClosed(() => {
        throw new Error('the resolver failed');
      }),
    ).rejects.toThrow('the resolver failed');
    // The close was not noted, so an unrelated error is not taken for it.
    await expect(
      untilClosed(() => Promise.reject(new Error('the resolver failed')), context),
    ).rejects.toThrow('the resolver failed');
  });

  test('MV-001: once a noted context has closed, whatever the closing itself raised is dropped', async ({ browser }) => {
    const context = await browser.newContext();
    noteClosing(context);
    await context.close();
    await untilClosed(() => Promise.reject(new Error('socket reset by the closing page')), context);
  });
});
