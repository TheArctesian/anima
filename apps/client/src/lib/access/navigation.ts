import { beforeNavigate, goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { readAccessToken } from '$lib/desktop/token';
import type { Hub } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { accessApi } from './api';
import { isBare, routePath, SIGN_IN_PATH, startGuard } from './guard';
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

/** Reload the whole client, so that a new hub address takes effect (WP-16 H-1). */
export function reloadClient(): void {
  location.reload();
}

/** The path the router matches for `url`. `resolve('/')` is the base path and a slash (SvelteKit's `base` is deprecated). */
function routeOf(url: URL): string {
  return routePath(url.pathname, resolve('/').slice(0, -1));
}

/** How the client starts on the page at `url`, the router's URL of the page the browser loaded (E-2, E-3). */
export function startOf(url: URL): ClientStart {
  return clientStart({ path: routeOf(url), desktop: capabilities().keyStore, storage: localStorage, origin: url.origin });
}

/** The start guard with the real router, key store and hub; the root layout calls this once, on a shell start. */
export function guardStart(url: URL, hub: Hub): void {
  const { me } = accessApi(hub);
  const check = { pathname: routeOf(url), address: hub.address, token: readAccessToken, desktop: capabilities().keyStore, me };
  startGuard({ ...check, goToSignIn }).catch(failed);
}

/**
 * The root layout decides bare or shell once, at start, so an in-app navigation that crosses between a bare route and
 * the operator shell loads the whole client at the new address instead: an operator page never renders without its hub,
 * and a share page never inherits the operator's hub (SEC-002, E-3). Call once in the root layout, during setup.
 */
export function loadWholeAcrossBare(bare: boolean): void {
  beforeNavigate((navigation) => {
    // A navigation that leaves the page (another site, a reload) starts the client afresh anyway.
    const to = navigation.to?.url;
    if (navigation.willUnload || to === undefined) return;
    if (isBare(routeOf(to)) === bare) return;
    navigation.cancel();
    location.assign(to.href);
  });
}
