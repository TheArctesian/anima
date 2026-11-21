import type { HubError } from '../error';

// The gateway of the hosted hubs in a browser (HMD-059, HMD-061): it stands in front of the hub, keeps the paths below
// `/_door/` for itself and answers a request that it does not pass on with `{ error: { code, message } }`. This file is
// pure: what the web client reads of those answers, and what it decides from them.

/** The entry into the hub of the signed-in account: the gateway makes the session of the hub again, or starts a sign-in. */
export const GATEWAY_ENTER_PATH = '/_door/enter';
/** Ends the session of the gateway (a `POST` from a page of its own origin). */
export const SIGN_OUT_PATH = '/_door/sign-out';
/** A path of the gateway that is no route of it: only a gateway answers it with the code `no_such_route`. */
export const PROBE_PATH = '/_door/client';

const CODES = ['sign_in_needed', 'hub_not_ready', 'hub_failed', 'too_many_requests', 'no_such_route'] as const;

export type GatewayCode = (typeof CODES)[number];

function isObject(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** The code of an answer that the gateway made itself; `null` for an answer of a hub and for any other body. */
export function gatewayCode(body: unknown): GatewayCode | null {
  if (!isObject(body) || !isObject(body.error)) return null;
  const code = body.error.code;
  return CODES.find((known) => known === code) ?? null;
}

const MESSAGES: Record<GatewayCode, HubError> = {
  // The same code as a hub that refuses the cookie, so each page sends the browser to the sign-in (API-003).
  sign_in_needed: { code: 'UNAUTHENTICATED', message: 'This browser is signed out. Sign in again.' },
  hub_not_ready: { code: 'UNAVAILABLE', message: 'Your hub is starting. Try again in a few seconds.' },
  hub_failed: { code: 'UNAVAILABLE', message: 'Your hub did not answer. Try again.' },
  too_many_requests: { code: 'UNAVAILABLE', message: 'Too many requests. Wait a minute.' },
  no_such_route: { code: 'malformed', message: "Can't read the hub's answer." },
};

/** A refusal of the gateway as the error of a hub call, in plain words. */
export function gatewayError(code: GatewayCode): HubError {
  return MESSAGES[code];
}

/** Whether the gateway refused because of the hub: the hub then counts as not reached (NET-004). */
export function hubDidNotAnswer(code: GatewayCode): boolean {
  return code === 'hub_not_ready' || code === 'hub_failed';
}

/** Whether the answer to `PROBE_PATH` came from a gateway. A hub alone has no such answer. */
export function isGatewayAnswer(status: number, body: unknown): boolean {
  return status === 404 && gatewayCode(body) === 'no_such_route';
}

/** How long after an entry the browser does not enter again by itself: a hub that takes no session must not loop. */
export const ENTER_AGAIN_MS = 60_000;

/** Whether the browser may go to the entry by itself now; `lastMs` is its last entry, `null` when there was none. */
export function mayEnterGatewayNow(lastMs: number | null, nowMs: number): boolean {
  return lastMs === null || nowMs - lastMs >= ENTER_AGAIN_MS || nowMs < lastMs;
}

/**
 * Ends the session of the gateway at `origin`. True when the gateway answered with its redirect (which the page does
 * not follow: `form-action 'none'` allows no form, and the place it names is of a different origin) or with success.
 */
export async function leaveGateway(send: typeof fetch, origin: string): Promise<boolean> {
  try {
    const answer = await send(`${origin}${SIGN_OUT_PATH}`, { method: 'POST', redirect: 'manual', credentials: 'same-origin' });
    return answer.type === 'opaqueredirect' || answer.ok || answer.status === 303;
  } catch {
    // No answer: the session of the gateway may still be open.
    return false;
  }
}

/** Asks whether a gateway stands in front of the hub at `origin`. False when nothing answers. */
export async function askGateway(send: typeof fetch, origin: string): Promise<boolean> {
  try {
    const answer = await send(`${origin}${PROBE_PATH}`, { headers: { accept: 'application/json' }, credentials: 'same-origin' });
    const body: unknown = await answer.json();
    return isGatewayAnswer(answer.status, body);
  } catch {
    // No answer, or a page in place of JSON: a hub alone.
    return false;
  }
}

/** Whether a gateway stands in front of the hub that served this page at `origin`, asked over the network of the browser. */
export function gatewayInFront(origin: string): Promise<boolean> {
  return askGateway(fetch, origin);
}

/** Ends the session of the gateway at `origin` over the network of the browser; see `leaveGateway`. */
export function gatewaySignOut(origin: string): Promise<boolean> {
  return leaveGateway(fetch, origin);
}
