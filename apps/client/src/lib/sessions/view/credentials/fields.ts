import type { RemoteProtocol } from '../../api';

/** A connection parameter the gateway may ask the operator for (GW-015, K4 §4). */
export type CredentialName = 'username' | 'domain' | 'password';

/** One field of the credential prompt. */
export interface CredentialField {
  readonly name: CredentialName;
  readonly label: string;
  /** Typed into a password input and offered for "Save password". */
  readonly secret: boolean;
}

/** The names the hub relays in `argv` for each protocol, in the order the prompt shows them (K4 §4). */
const ALLOWED: Readonly<Record<RemoteProtocol, readonly CredentialName[]>> = {
  RDP: ['username', 'domain', 'password'],
  VNC: ['username', 'password'],
};

const FIELDS: Readonly<Record<CredentialName, CredentialField>> = {
  username: { name: 'username', label: 'Username', secret: false },
  domain: { name: 'domain', label: 'Domain', secret: false },
  password: { name: 'password', label: 'Password', secret: true },
};

/**
 * The fields to ask for a `required` instruction, in a fixed order whatever order the gateway used; undefined
 * when the prompt cannot satisfy it, and the caller ends the session.
 *
 * It cannot when `required` names nothing, or names any parameter outside the protocol's allowed set. Asking
 * only the known names would leave the gateway waiting for a value that never comes (the hub drops `argv` for
 * other names), and showing the gateway's own names would let it ask the operator for anything. Ending the
 * session fails closed and says so.
 */
export function askedFields(protocol: RemoteProtocol, required: readonly string[]): readonly CredentialField[] | undefined {
  const allowed: readonly string[] = ALLOWED[protocol];
  if (required.length === 0 || required.some((name) => !allowed.includes(name))) return undefined;
  return ALLOWED[protocol].filter((name) => required.includes(name)).map((name) => FIELDS[name]);
}
