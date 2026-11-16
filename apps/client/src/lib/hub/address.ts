/** Where the hub answers: the origin the operator sees, and the two GraphQL endpoints. */
export interface HubAddress {
  /** `https://host[:port]` (`http://` only for this computer), shown to the operator (NET-004). */
  readonly origin: string;
  /** HTTP endpoint for queries and mutations. */
  readonly http: string;
  /** WebSocket endpoint for subscriptions (API-002). */
  readonly socket: string;
}

type AddressProblem = 'not-a-url' | 'not-https' | 'has-credentials' | 'has-path';

/** Why an address was refused. The message never repeats the input, which may hold a password. */
interface AddressError {
  readonly problem: AddressProblem;
  readonly message: string;
}

type AddressResult = { readonly ok: true; readonly value: HubAddress } | { readonly ok: false; readonly error: AddressError };

/** The default hub (NET-005, DNS-001). */
export const DEFAULT_HUB_ADDRESS: HubAddress = Object.freeze({
  origin: 'https://hub.anima.ing',
  http: 'https://hub.anima.ing/graphql',
  socket: 'wss://hub.anima.ing/graphql',
});

/** Hosts that may use plain `http` and `ws`: they never leave this computer (CLI-002, F-1). */
// Only `127.0.0.1`. No `[::1]`: a CSP source cannot name an IPv6 literal. No `localhost`: it may resolve to ::1 first, where
// another user of this computer can bind the same port (security review of S6 T6).
const LOOPBACK_HOSTS: readonly string[] = ['127.0.0.1'];

const MESSAGES: Record<AddressProblem, string> = {
  'not-a-url': 'Type a host name, for example hub.anima.ing.',
  'not-https': 'The hub address must use https. Only this computer may use http.',
  'has-credentials': 'Remove the user name and password from the hub address.',
  'has-path': 'Type only the host name and port, no path.',
};

function refused(problem: AddressProblem): AddressResult {
  return { ok: false, error: { problem, message: MESSAGES[problem] } };
}

/** Parse an address the operator typed. A bare host name means `https://` (NET-005); `http://` is only for `127.0.0.1` (CLI-002, F-1). */
export function parseHubAddress(text: string): AddressResult {
  const trimmed = text.trim();
  const withScheme = trimmed.includes('://') ? trimmed : `https://${trimmed}`;
  if (trimmed === '' || !URL.canParse(withScheme)) return refused('not-a-url');
  const url = new URL(withScheme);
  const plain = url.protocol === 'http:' && LOOPBACK_HOSTS.includes(url.hostname);
  if (url.protocol !== 'https:' && !plain) return refused('not-https');
  if (url.username !== '' || url.password !== '') return refused('has-credentials');
  if (url.pathname !== '/' || url.search !== '' || url.hash !== '') return refused('has-path');
  return {
    ok: true,
    value: Object.freeze({ origin: url.origin, http: `${url.origin}/graphql`, socket: `${plain ? 'ws' : 'wss'}://${url.host}/graphql` }),
  };
}
