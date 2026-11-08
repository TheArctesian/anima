import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { readAccessToken } from '$lib/desktop/token';
import type { HubAddress } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { SIGN_IN_PATH, startGuard } from './guard';

// The browser side of access: the router, the page reload and the desktop check. Logic stays in `guard.ts`.

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

/** The start guard with the real router and key store; the root layout calls this once. */
export function guardStart(pathname: string, address: HubAddress): void {
  startGuard({ pathname, address, token: readAccessToken, desktop: capabilities().keyStore, goToSignIn }).catch(failed);
}
