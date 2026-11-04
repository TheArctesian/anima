/** Where the hub answers: the origin the operator sees, and the two GraphQL endpoints. */
export interface HubAddress {
  /** `https://host[:port]`, shown to the operator (NET-004). */
  readonly origin: string;
  /** HTTP endpoint for queries and mutations. */
  readonly http: string;
  /** WebSocket endpoint for subscriptions (API-002). */
  readonly socket: string;
}

export type AddressProblem = 'not-a-url' | 'not-https' | 'has-credentials' | 'has-path';

/** Why an address was refused. The message never repeats the input, which may hold a password. */
export interface AddressError {
  readonly problem: AddressProblem;
  readonly message: string;
}

export type AddressResult = { readonly ok: true; readonly value: HubAddress } | { readonly ok: false; readonly error: AddressError };

/** The default hub (NET-005, DNS-001). */
export const DEFAULT_HUB_ADDRESS: HubAddress = {
  origin: 'https://hub.anima.ing',
  http: 'https://hub.anima.ing/graphql',
  socket: 'wss://hub.anima.ing/graphql',
};

const MESSAGES: Record<AddressProblem, string> = {
  'not-a-url': 'Type the hub address as a host name, for example hub.anima.ing.',
  'not-https': 'The hub address must use https.',
  'has-credentials': 'Remove the user name and password from the hub address.',
  'has-path': 'Type only the host name and port of the hub, without a path.',
};

function refused(problem: AddressProblem): AddressResult {
  return { ok: false, error: { problem, message: MESSAGES[problem] } };
}

/** Parse an address the operator typed. A bare host name means `https://` (NET-005). */
export function parseHubAddress(text: string): AddressResult {
  const trimmed = text.trim();
  const withScheme = trimmed.includes('://') ? trimmed : `https://${trimmed}`;
  if (trimmed === '' || !URL.canParse(withScheme)) return refused('not-a-url');
  const url = new URL(withScheme);
  if (url.protocol !== 'https:') return refused('not-https');
  if (url.username !== '' || url.password !== '') return refused('has-credentials');
  if (url.pathname !== '/' || url.search !== '' || url.hash !== '') return refused('has-path');
  return {
    ok: true,
    value: { origin: url.origin, http: `${url.origin}/graphql`, socket: `wss://${url.host}/graphql` },
  };
}
