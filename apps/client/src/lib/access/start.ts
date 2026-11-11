import { storedHubAddress } from '$lib/desktop/address';
import type { HubAddress } from '$lib/hub/hub';
import { servedHubAddress } from '$lib/platform/browser';

/**
 * How the client starts. `refused`: a browser page whose origin cannot be a hub, so nothing is sent anywhere (E-2).
 * `shell`: the operator shell, talking to `address`.
 */
export type ClientStart =
  | { readonly kind: 'refused'; readonly message: string }
  | { readonly kind: 'shell'; readonly address: HubAddress };

/** What the start decision reads; the caller reads the platform and the browser (CQ-021). */
interface StartPlace {
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
  if (place.desktop) return { kind: 'shell', address: storedHubAddress(place.storage) };
  const served = servedHubAddress(place.origin);
  return served === null ? { kind: 'refused', message: ORIGIN_REFUSED } : { kind: 'shell', address: served };
}
