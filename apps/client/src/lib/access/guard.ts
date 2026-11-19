import type { HubAddress, HubError, HubResult, TokenProvider } from '$lib/hub/hub';
import { isSignedOut } from './routes';

/** Takes the client to the sign-in page. Injected, so that this file stays pure (WP-15 D-4). */
type GoToSignIn = () => void;

/** What the desktop app knows at its start for the first-run decision (FRN-001, NAV-009). */
interface FirstRunSign {
  /** Whether a hub address is saved (`anima.hubAddress`). */
  readonly addressSaved: boolean;
  /** Takes the client to the first-run choices. */
  readonly goToWelcome: () => void;
}

/** What the start guard needs; every part is injected so that tests need no desktop shell or hub. */
interface StartCheck {
  /** The route path (see `routePath`). */
  readonly route: string;
  readonly address: HubAddress;
  /** The desktop key store's reader. */
  readonly token: TokenProvider;
  /** Whether the key store exists: the desktop app reads its token, a browser asks the hub (WP-15 D-4, E-3). */
  readonly desktop: boolean;
  /** Who the browser's cookie belongs to (the access api's `me`). */
  readonly me: () => Promise<HubResult<unknown>>;
  readonly goToSignIn: GoToSignIn;
  /** The desktop app's first-run sign; absent, a missing token always goes to sign-in. */
  readonly firstRun?: FirstRunSign;
}

/**
 * Whether this start needs the account sign-in screen (HMD-022): the desktop app has no chosen hub.
 * An older installation's token for the default hub does not choose a hub for this installation.
 */
export function isFirstRun(desktop: boolean, addressSaved: boolean): boolean {
  return desktop && !addressSaved;
}

/**
 * Goes to sign-in when the client is not signed in to this hub, except on a signed-out route (API-003, H-5, E-3), or
 * to the first-run choices on the desktop app's first run (FRN-001). The desktop app looks in its key store. A browser
 * cannot read its `HttpOnly` cookie, so it asks the hub `me`: only UNAUTHENTICATED goes to sign-in; any other failure
 * stays, and the hub status shows it (NET-004).
 */
export async function startGuard(check: StartCheck): Promise<void> {
  if (isSignedOut(check.route)) return;
  if (check.firstRun !== undefined && isFirstRun(check.desktop, check.firstRun.addressSaved)) {
    check.firstRun.goToWelcome();
    return;
  }
  if (!check.desktop) {
    const result = await check.me();
    if (!result.ok) signInNeeded(result.error, check.goToSignIn);
    return;
  }
  // Once a hub is chosen, a refused key store or listener check goes to that hub's sign-in.
  const read = await check.token(check.address).then(
    (token) => ({ ok: true as const, token }),
    () => ({ ok: false as const }),
  );
  if (read.ok && read.token !== null && read.token !== '') return;
  check.goToSignIn();
}

/** Pages call this on a failed hub result: goes to sign-in for `UNAUTHENTICATED` and says whether it did (API-003). */
export function signInNeeded(error: HubError, goToSignIn: GoToSignIn): boolean {
  if (error.code !== 'UNAUTHENTICATED') return false;
  goToSignIn();
  return true;
}
