import type { HubAddress, HubError, HubResult, TokenProvider } from '$lib/hub/hub';

/** Takes the client to the sign-in page. Injected, so that this file stays pure (WP-15 D-4). */
type GoToSignIn = () => void;

/** The sign-in page path. */
export const SIGN_IN_PATH = '/sign-in';

/** The share viewer's page (D-7). */
const SHARED_PATH = '/shared';

/** The routes that need no sign-in (API-003, E-3): the start guard never asks the hub about them. */
const SIGNED_OUT_ROUTES = [SIGN_IN_PATH, SHARED_PATH] as const;

/**
 * The signed-out routes that render only their page: no operator hub, socket, guard or shell, so a share viewer calls
 * no operator operation (SEC-002, D-7). `/sign-in` is signed out but keeps the shell: its mutation needs the hub.
 */
const BARE_ROUTES = [SHARED_PATH] as const;

/** Whether `path` is `route` or below it by whole segment: `/shared/x` is below `/shared`, `/sharedx` is not. */
function within(path: string, route: string): boolean {
  return path === route || path.startsWith(`${route}/`);
}

/** Whether the route path (see `routePath`) needs no sign-in (API-003, E-3). */
export function isSignedOut(path: string): boolean {
  return SIGNED_OUT_ROUTES.some((route) => within(path, route));
}

/** Whether the route path (see `routePath`) renders bare, with no operator hub (SEC-002, D-7, E-3). */
export function isBare(path: string): boolean {
  return BARE_ROUTES.some((route) => within(path, route));
}

/**
 * The path the router matches for `pathname`: below the `base` path and percent-decoded as SvelteKit decodes it
 * (`%25` stays), so `/%73hared` counts as `/shared` exactly when the router shows the share page there. A path the router
 * cannot decode stays as it is; the router shows its error page for it.
 */
export function routePath(pathname: string, base: string): string {
  const below = base !== '' && within(pathname, base) ? pathname.slice(base.length) : pathname;
  try {
    return below.split('%25').map(decodeURI).join('%25') || '/';
  } catch {
    // A malformed escape: the router refuses it too, so the undecoded text decides.
    return below || '/';
  }
}

/** What the start guard needs; every part is injected so that tests need no desktop shell or hub. */
interface StartCheck {
  /** The route path (see `routePath`). */
  readonly pathname: string;
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
  if (isSignedOut(check.pathname)) return;
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
