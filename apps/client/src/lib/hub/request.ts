import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import type { HubAddress } from './address';
import { isGraphQLResponse, MALFORMED, parseResponse, TOKEN_UNREADABLE, UNREACHABLE, type HubResult } from './error';
import type { StatusSignal } from './status/status';

/**
 * Reads the operator's access token for one hub, or `null` before login. The token is read per call, never stored here.
 * The hub passes its own address, so a provider cannot be bound to a different hub than the one it is sent to (CLI-002, CLI-003).
 */
export type TokenProvider = (address: HubAddress) => Promise<string | null>;

/** What one HTTP GraphQL call needs; every part is injected so that tests need no network (CQ-021). */
interface HttpTransport {
  readonly address: HubAddress;
  readonly endpoint: string;
  readonly fetch: typeof fetch;
  readonly token: TokenProvider;
  readonly signal: (signal: StatusSignal) => void;
  /** How long to wait for a complete answer before the hub counts as unreachable (NET-004). */
  readonly timeoutMs: number;
}

/** The token, or `TOKEN_UNREADABLE` when the provider rejects; nothing from the rejection is kept (LOG-007). */
export async function readToken(token: TokenProvider, address: HubAddress): Promise<HubResult<string | null>> {
  try {
    return { ok: true, value: await token(address) };
  } catch {
    // The reason may describe the key store; the operator only needs to know the token is missing.
    return { ok: false, error: TOKEN_UNREADABLE };
  }
}

function headersFor(token: string | null): Record<string, string> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    accept: 'application/graphql-response+json, application/json',
  };
  if (token !== null) headers.authorization = `Bearer ${token}`;
  return headers;
}

/** The answer text, or `undefined` when no complete answer arrived within the timeout. */
async function answerText(transport: HttpTransport, token: string | null, body: string): Promise<string | undefined> {
  const abort = new AbortController();
  const timer = setTimeout(() => abort.abort(), transport.timeoutMs);
  try {
    const init = { method: 'POST', headers: headersFor(token), body, signal: abort.signal };
    const response = await transport.fetch(transport.endpoint, init);
    return await response.text();
  } catch {
    // fetch rejects only when no answer arrived (network, TLS, reset, timeout); the reason is the same for the operator.
    return undefined;
  } finally {
    clearTimeout(timer);
  }
}

function parseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    // Not JSON: the caller reports `malformed`.
    return undefined;
  }
}

/**
 * POST `document` with `variables` to the hub (HTTP status is ignored; the GraphQL body decides).
 * Always resolves, with a typed result for every hub, network and token outcome.
 */
export async function sendRequest<TResult, TVariables>(
  transport: HttpTransport,
  document: TypedDocumentNode<TResult, TVariables>,
  variables: TVariables,
): Promise<HubResult<TResult>> {
  const token = await readToken(transport.token, transport.address);
  if (!token.ok) return token;
  const text = await answerText(transport, token.value, JSON.stringify({ query: print(document), variables }));
  if (text === undefined) {
    transport.signal('failed');
    return { ok: false, error: UNREACHABLE };
  }
  const body = parseJson(text);
  if (!isGraphQLResponse(body)) return { ok: false, error: MALFORMED };
  transport.signal('answered');
  const result = parseResponse(body);
  // The document was checked against the schema at build time (API-008); the hub answers in that shape.
  return result.ok ? { ok: true, value: result.value as TResult } : result;
}
