import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { hasStoredHubAddress, storedHubAddress } from '$lib/desktop/address';
import { behindGateway } from '$lib/hosted/gateway/here';
import { holdsMovedAddressHere, hubIsHosted } from '$lib/hosted/mark';
import type { Hub } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { accessApi } from './api';
import { startGuard } from './guard';
import { tokenFor } from './listener/check';
import { GATEWAY_SIGN_IN_PATH, isSignedOut, routePath, signInPathFor, SIGN_IN_PATH, WELCOME_PATH } from './routes';
import { clientStart, type ClientStart } from './start';

// The browser side of access: the router, the page reload, the platform and the page origin. Logic stays in
// `guard.ts` and `start.ts`.

function failed(): void {
  // goto rejects only when navigation itself breaks; the operator stays on the page they were on.
  console.warn('Navigation failed.');
}

/** The sign-in of this client's hub: the password page, or the account for a hosted hub (R-129). */
export function goToSignIn(): void {
  // The desktop app with no saved address has no hub to sign in to: it shows the first-run choices (R-225).
  if (capabilities().keyStore && !hasStoredHubAddress(localStorage)) return goToWelcome();
  // The address of before is the hosted service now: the first-run choices lead to the account or to a hub address (HM-T31).
  if (capabilities().keyStore && holdsMovedAddressHere(storedHubAddress(localStorage).origin)) return goToWelcome();
  // A browser signs in to the hub that served it: with the hub password, or, behind the gateway of the hosted hubs,
  // through the gateway and the account (HMD-059). Never the account sign-in of the desktop app.
  if (!capabilities().keyStore) {
    void behindGateway()
      .then((behind) => goto(resolve(behind ? GATEWAY_SIGN_IN_PATH : SIGN_IN_PATH)))
      .catch(failed);
    return;
  }
  const hosted = hubIsHosted(storedHubAddress(localStorage).origin);
  goto(resolve(signInPathFor(hosted))).catch(failed);
}

/** The first-run choices of the desktop app (FRN-001). */
export function goToWelcome(): void {
  goto(resolve(WELCOME_PATH)).catch(failed);
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
export function loadClientAt(path: '/devices' | typeof SIGN_IN_PATH | typeof WELCOME_PATH): void {
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

/**
 * The start guard with the real router, key store and hub; the root layout calls this once, on a shell start. On this
 * computer's hub address the token is read only after the listener check (F-10), so a refusal leads to sign-in, which
 * says why; with no address and no token the desktop app shows the first-run choices (FRN-001).
 */
export function guardStart(url: URL, hub: Hub): void {
  const { me } = accessApi(hub);
  const desktop = capabilities().keyStore;
  const check = { route: routeOf(url), address: hub.address, token: tokenFor(hub.address, desktop), desktop, me };
  const firstRun = { addressSaved: hasStoredHubAddress(localStorage), goToWelcome };
  startGuard({ ...check, goToSignIn, firstRun }).catch(failed);
}
