import { parseHubAddress, type HubAddress } from '$lib/hub/hub';
import type { Capabilities } from './capabilities';

/** The answers of a client running in a browser: none of the desktop-only features (CLI-001, UPD-001). */
export const browserCapabilities: Capabilities = {
  keyStore: false,
  webViewTabs: false,
  popOut: false,
  updates: false,
  processLog: false,
  thisMachine: false,
  systemSettings: false,
  builds: false,
};

/** Hosts a plain `http:` page may come from: this machine only, for a local build or test (E-2). */
const LOOPBACK_HOSTS: readonly string[] = ['localhost', '127.0.0.1', '[::1]'];

/**
 * The hub a browser talks to: the origin that served the page (CLI-002, E-2), with `wss:` for an `https:` origin and
 * `ws:` for a loopback `http:` origin. Any other origin answers `null`: the hub serves the web client only over TLS
 * (SEC-001), so such a page did not come from a hub, and its `Secure` cookie could never be set there (CLI-003);
 * sending the sign-in form to it would only put the password on the network in clear text.
 */
export function servedHubAddress(origin: string): HubAddress | null {
  if (!URL.canParse(origin)) return null;
  const url = new URL(origin);
  if (url.protocol === 'https:') {
    const parsed = parseHubAddress(url.origin);
    return parsed.ok ? parsed.value : null;
  }
  if (url.protocol !== 'http:' || !LOOPBACK_HOSTS.includes(url.hostname)) return null;
  return Object.freeze({ origin: url.origin, http: `${url.origin}/graphql`, socket: `ws://${url.host}/graphql` });
}
