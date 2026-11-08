import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { readAccessToken } from '$lib/desktop/token';
import type { Hub } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { accessApi } from './api';
import { startGuard } from './guard';
import { routePath, SIGN_IN_PATH } from './routes';
import { clientStart, startHolds, type ClientStart } from './start';

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
 * Reload the whole client at the address in the address bar, so that it starts afresh there: a new hub address takes
 * effect (WP-16 H-1), and a page the router shows across the bare line gets the start it needs (SEC-002, E-3). The
 * router writes the address before it renders, so the reload lands on the page it was showing. `location.replace`
 * with that address would not reload: an address that differs only by its fragment (`/shared#token`) is a jump
 * within the same document.
 */
export function reloadClient(): void {
  location.reload();
}

/** The path the router matches for `url`. `resolve('/')` is the base path and a slash (SvelteKit's `base` is deprecated). */
function routeOf(url: URL): string {
  return routePath(url.pathname, resolve('/').slice(0, -1));
}

/** How the client starts on the page the browser loaded, at the router's `url` (E-2, E-3); the root layout calls this once. */
export function startAt(url: URL): ClientStart {
  return clientStart({ route: routeOf(url), desktop: capabilities().keyStore, storage: localStorage, origin: url.origin });
}

/** The start guard with the real router, key store and hub; the root layout calls this once, on a shell start. */
export function guardStart(url: URL, hub: Hub): void {
  const { me } = accessApi(hub);
  const check = { route: routeOf(url), address: hub.address, token: readAccessToken, desktop: capabilities().keyStore, me };
  startGuard({ ...check, goToSignIn }).catch(failed);
}

/** Whether the page the router shows at `url` belongs on `start`; read it in the template so it follows the router. */
export function holdsAt(start: ClientStart, url: URL): boolean {
  return startHolds(start, routeOf(url));
}
