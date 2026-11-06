import { sendArgument, type ArgumentClient } from '../../tunnel/argv';
import type { CredentialField, CredentialName } from './fields';

/** What the operator typed, by parameter name. */
export type Answers = Readonly<Partial<Record<CredentialName, string>>>;

/**
 * Send each asked field's value to the gateway once, in its own argument value stream (GW-015). A field left empty
 * is sent empty, so the gateway is not left waiting; names outside `fields` are never sent.
 */
export function sendArguments(client: ArgumentClient, fields: readonly CredentialField[], answers: Answers): void {
  for (const { name } of fields) sendArgument(client, name, answers[name] ?? '');
}
