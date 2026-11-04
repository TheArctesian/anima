import type { HubAddress, HubError, TokenProvider } from '$lib/hub/hub';

/** Takes the client to the sign-in page. Injected, so that this file stays pure (WP-15 D-4). */
export type GoToSignIn = () => void;

/** The sign-in page path. */
export const SIGN_IN_PATH = '/sign-in';

/** What the start guard needs; every part is injected so that tests need no desktop shell. */
export interface StartCheck {
  readonly pathname: string;
  readonly address: HubAddress;
  /** The desktop key store's reader. */
  readonly token: TokenProvider;
  /** Whether the key store exists. The browser has no token provider, so the guard does nothing there (WP-15 D-4). */
  readonly desktop: boolean;
  readonly goToSignIn: GoToSignIn;
}

/** Goes to sign-in when the desktop app has no usable token for this hub and the client is not already there (API-003, H-5). */
export async function startGuard(check: StartCheck): Promise<void> {
  if (!check.desktop || check.pathname === SIGN_IN_PATH) return;
  const token = await check.token(check.address).catch(() => null);
  if (token === null || token === '') check.goToSignIn();
}

/** Pages call this on a failed hub result: goes to sign-in for `UNAUTHENTICATED` and says whether it did (API-003). */
export function signInNeeded(error: HubError, goToSignIn: GoToSignIn): boolean {
  if (error.code !== 'UNAUTHENTICATED') return false;
  goToSignIn();
  return true;
}
