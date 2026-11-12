import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { readAccessToken } from '$lib/desktop/token';
import type { Hub } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { accessApi } from './api';
import { startGuard } from './guard';
import { isSignedOut, routePath, SIGN_IN_PATH } from './routes';
import { clientStart, type ClientStart } from './start';

// The browser side of access: the router, the page reload, the platform and the page origin. Logic stays in
// `guard.ts` and `start.ts`.

function failed(): void {
  // goto rejects only when navigation itself breaks; the operator stays on the page they were on.
  console.warn('Navigation failed.');
}

export function goToSignIn(): void {
  goto(resolve(SIGN_IN_PATH)).catch(failed);
}

export function goToDevices(): void {
  goto(resolve('/devices')).catch(failed);
}

/**
 * Reload the whole client at the address in the address bar, so that it starts afresh there and a new hub address
 * takes effect (WP-16 H-1). `location.replace` with that address would not reload: an address that differs only by
 * its fragment is a jump within the same document.
 */
export function reloadClient(): void {
  location.reload();
}

/**
 * Load the whole client at `path` as a new page, not a router move (E-4). After a browser sign-in or sign-out the hub has
 * set or cleared the `HttpOnly` cookie, and only a new page opens its socket with the cookie as it is now, so no socket
 * signed by an old cookie survives. `replace`: the page left behind does not stay in the history. The path always
 * differs from the current one, so this is a load and never a jump within the page.
 */
export function loadClientAt(path: '/devices' | typeof SIGN_IN_PATH): void {
  location.replace(resolve(path));
}

/** The path the router matches for `url`. `resolve('/')` is the base path and a slash (SvelteKit's `base` is deprecated). */
function routeOf(url: URL): string {
  return routePath(url.pathname, resolve('/').slice(0, -1));
}

/** Whether `url` is a page that needs no sign-in, so the window shows that page alone (NAV-009). */
export function showsSignedOutPage(url: URL): boolean {
  return isSignedOut(routeOf(url));
}

/** How the client starts on the page the browser loaded, at the router's `url` (E-2); the root layout calls this once. */
export function startAt(url: URL): ClientStart {
  return clientStart({ desktop: capabilities().keyStore, storage: localStorage, origin: url.origin });
}

/** The start guard with the real router, key store and hub; the root layout calls this once, on a shell start. */
export function guardStart(url: URL, hub: Hub): void {
  const { me } = accessApi(hub);
  const check = { route: routeOf(url), address: hub.address, token: readAccessToken, desktop: capabilities().keyStore, me };
  startGuard({ ...check, goToSignIn }).catch(failed);
}
