import type { HubAddress, HubError, HubResult, TokenProvider } from '$lib/hub/hub';
import { isSignedOut } from './routes';

/** Takes the client to the sign-in page. Injected, so that this file stays pure (WP-15 D-4). */
type GoToSignIn = () => void;

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
}

/**
 * Goes to sign-in when the client is not signed in to this hub, except on a signed-out route (API-003, H-5, E-3). The
 * desktop app looks in its key store. A browser cannot read its `HttpOnly` cookie, so it asks the hub `me`: only
 * UNAUTHENTICATED goes to sign-in; any other failure stays, and the hub status shows it (NET-004).
 */
export async function startGuard(check: StartCheck): Promise<void> {
  if (isSignedOut(check.route)) return;
  if (!check.desktop) {
    const result = await check.me();
    if (!result.ok) signInNeeded(result.error, check.goToSignIn);
    return;
  }
  const token = await check.token(check.address).catch(() => null);
  if (token === null || token === '') check.goToSignIn();
}

/** Pages call this on a failed hub result: goes to sign-in for `UNAUTHENTICATED` and says whether it did (API-003). */
export function signInNeeded(error: HubError, goToSignIn: GoToSignIn): boolean {
  if (error.code !== 'UNAUTHENTICATED') return false;
  goToSignIn();
  return true;
}
