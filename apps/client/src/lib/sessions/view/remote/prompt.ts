import type { RemoteProtocol } from '../../api';
import { askedFields, type CredentialField } from '../credentials/fields';

/** What the gateway's last `required` instruction asks of the view now (GW-015). */
export type GatewayAsk =
  | { readonly kind: 'none' }
  /** Show the credential prompt for these fields; `required` is the instruction it answers. */
  | { readonly kind: 'ask'; readonly fields: readonly CredentialField[]; readonly required: readonly string[] }
  /** The prompt cannot answer it, so the session ends (it fails closed). */
  | { readonly kind: 'refuse' };

const NONE: GatewayAsk = { kind: 'none' };

/**
 * The ask for `required`, the names of the last `required` instruction; `answered` is the instruction the operator
 * already answered, which asks nothing more.
 */
export function gatewayAsk(protocol: RemoteProtocol, required: readonly string[] | undefined, answered: readonly string[] | undefined): GatewayAsk {
  if (required === undefined || required === answered) return NONE;
  const fields = askedFields(protocol, required);
  return fields === undefined ? { kind: 'refuse' } : { kind: 'ask', fields, required };
}

/** Fixed texts of a session the prompt ended; never the gateway's names (LOG-007, R-035). */
export const PROMPT_CANCELLED = 'You cancelled the sign-in, so the session ended.';
export const PROMPT_REFUSED = 'The device asked for a sign-in Anima cannot give, so the session ended.';
