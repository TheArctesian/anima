import { randomBytes } from 'node:crypto';

// How the hub tells who is calling (K7 "Authentication transport"), as the browser tests need it.

export const HUB_HOST = 'hub.anima.ing';
export const HUB_ORIGIN = `https://${HUB_HOST}`;
export const SESSION_COOKIE = '__Host-anima';
/** Seven days, as K7 says. */
export const COOKIE_MAX_AGE_SECONDS = 604_800;

/** The bearer value `login` hands out; the hub accepts it, as it would a real access token. */
export const BEARER_TOKEN = 'synthetic-bearer-not-a-secret';

/** The one operator the synthetic hub knows. Nothing here is a real credential (CON-003). */
export const OPERATOR = { username: 'ann', password: 'correct-horse-synthetic', code: '123456' } as const;

/** What a request shows about itself. The cookie value is read from the context's cookie jar, never from a header. */
export interface RequestFacts {
  readonly origin: string | null;
  /** The `Authorization` header, if any. Used to decide, never kept or printed. */
  readonly authorization: string | null;
  readonly cookie: string | null;
}

export type Caller = 'cookie' | 'bearer' | 'signed-out';

/** The browser sessions the hub has issued; a cookie counts only while its token is here. */
export interface BrowserSessions {
  issue(): string;
  revoke(token: string): void;
  isIssued(token: string): boolean;
}

export function browserSessions(): BrowserSessions {
  const issued = new Set<string>();
  return {
    issue() {
      const token = randomBytes(32).toString('base64url');
      issued.add(token);
      return token;
    },
    revoke: (token) => issued.delete(token),
    isIssued: (token) => issued.has(token),
  };
}

/**
 * An `Authorization` header wins and the cookie is then ignored: the request is signed in only if the bearer value is good.
 * The cookie counts only without an `Authorization` header and with an `Origin` equal to the public origin (K7).
 */
export function callerOf(facts: RequestFacts, sessions: BrowserSessions): Caller {
  if (facts.authorization !== null) return facts.authorization === `Bearer ${BEARER_TOKEN}` ? 'bearer' : 'signed-out';
  const cookieAllowed = facts.cookie !== null && facts.origin === HUB_ORIGIN && sessions.isIssued(facts.cookie);
  return cookieAllowed ? 'cookie' : 'signed-out';
}

/** `Set-Cookie` text for a good sign-in, and for a sign-out. */
export function sessionCookie(token: string): string {
  return `${SESSION_COOKIE}=${token}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=${COOKIE_MAX_AGE_SECONDS}`;
}

export function clearedCookie(): string {
  return `${SESSION_COOKIE}=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0`;
}

/** The root fields a signed-out request may run, one at a time (K7 "HTTP surface"). */
export const SIGNED_OUT_FIELDS: readonly string[] = ['login', 'signInBrowser', 'joinShare'];
