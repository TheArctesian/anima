import { expect, test } from '@playwright/test';
import { BEARER_TOKEN, browserSessions, callerOf, HUB_ORIGIN } from './synthetic/access';
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
