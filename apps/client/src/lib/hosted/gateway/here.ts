import { capabilities } from '$lib/platform/platform';
import { GATEWAY_ENTER_PATH, gatewayInFront, gatewaySignOut, mayEnterGatewayNow } from '$lib/hub/hub';
import { markHosted } from '../mark';

// The gateway as this browser page meets it: the one question per page load, the entry and the sign-out. Rules are in
// `$lib/hub/gateway`.

const ENTERED_KEY = 'anima.gatewayEntered';
const LEFT_KEY = 'anima.gatewayLeft';

let asked: Promise<boolean> | undefined;

function session(): Storage | null {
  try {
    return sessionStorage;
  } catch {
    // A privacy mode that refuses storage.
    return null;
  }
}

/**
 * Whether this page came through the gateway of the hosted hubs (HMD-059). Asked once for each page load, and never by
 * the desktop app, which reaches a hosted hub through its link. A hub behind the gateway is a hosted one, so it is
 * marked: no screen then names a VPN or a hub password for it (R-222, HMD-038).
 */
export function behindGateway(): Promise<boolean> {
  if (capabilities().keyStore) return Promise.resolve(false);
  asked ??= gatewayInFront(location.origin).then((behind) => {
    if (behind) {
      try {
        markHosted(localStorage, location.origin);
      } catch {
        // The mark only changes words; without storage the page still works.
      }
    }
    return behind;
  });
  return asked;
}

/** Whether this browser signed out here and has not entered since: the sign-in page then waits for the person. */
export function leftGateway(): boolean {
  return session()?.getItem(LEFT_KEY) === '1';
}

/** Whether the browser may go to the entry by itself: not after a sign-out, and not right after an entry (no loop). */
export function mayEnterByItself(now: number = Date.now()): boolean {
  if (leftGateway()) return false;
  const last = Number(session()?.getItem(ENTERED_KEY) ?? Number.NaN);
  return mayEnterGatewayNow(Number.isFinite(last) ? last : null, now);
}

/** Goes to the entry of the gateway: a new page, on which the gateway makes the session of the hub or starts a sign-in. */
export function enterGateway(now: number = Date.now()): void {
  const store = session();
  store?.removeItem(LEFT_KEY);
  store?.setItem(ENTERED_KEY, String(now));
  location.assign(GATEWAY_ENTER_PATH);
}

/**
 * Ends the session of the gateway after the hub ended its own (HMD-062). `none`: no gateway is in front of this hub.
 * `kept`: the gateway did not confirm, so this browser may still be signed in.
 */
export async function signOutOfGateway(): Promise<'none' | 'left' | 'kept'> {
  if (!(await behindGateway())) return 'none';
  if (!(await gatewaySignOut(location.origin))) return 'kept';
  session()?.setItem(LEFT_KEY, '1');
  return 'left';
}
