import { storedHubAddress } from '$lib/desktop/address';
import type { HubAddress } from '$lib/hub/hub';
import { servedHubAddress } from '$lib/platform/browser';
import { isBare } from './routes';

/**
 * How the client starts on a page. `bare`: a signed-out page alone, with no operator hub (SEC-002, D-7, E-3). `refused`:
 * a browser page whose origin cannot be a hub, so nothing is sent anywhere (E-2). `shell`: the operator shell, talking to
 * `address`.
 */
export type ClientStart =
  | { readonly kind: 'bare' }
  | { readonly kind: 'refused'; readonly message: string }
  | { readonly kind: 'shell'; readonly address: HubAddress };

/** What the start decision reads; the caller reads the router, the platform and the browser (CQ-021). */
interface StartPlace {
  /** The route path (see `routePath`). */
  readonly route: string;
  /** Whether the client is the desktop app, with a key store (WP-42 D-4). */
  readonly desktop: boolean;
  /** Where the desktop app keeps its hub address (CLI-002). */
  readonly storage: Storage;
  /** The origin that served the page. */
  readonly origin: string;
}

/** Fixed text: it never repeats the origin. */
export const ORIGIN_REFUSED = 'Anima runs only from the https address of its hub. Open the hub address in the browser.';

/**
 * Decides how the client starts. The desktop app talks to the hub in its settings; a browser talks only to the hub that
 * served the page and ignores any stored address, so script on the page cannot point it at another hub (CLI-002, E-2).
 */
export function clientStart(place: StartPlace): ClientStart {
  if (isBare(place.route)) return { kind: 'bare' };
  if (place.desktop) return { kind: 'shell', address: storedHubAddress(place.storage) };
  const served = servedHubAddress(place.origin);
  return served === null ? { kind: 'refused', message: ORIGIN_REFUSED } : { kind: 'shell', address: served };
}

/**
 * Whether the route the router shows now belongs on this start: a bare route on a bare start, any other route on a
 * shell or refused start. The start is decided once, but the router can move without asking (a navigation started
 * while another is in flight, a `load` redirect), so the layout checks what it shows, not only where it goes (SEC-002,
 * E-3).
 */
export function startHolds(start: ClientStart, route: string): boolean {
  return isBare(route) === (start.kind === 'bare');
}
