import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { isGraphQLResponse, MALFORMED, parseResponse, UNREACHABLE, type HubResult } from './error';
import type { StatusSignal } from './status/status';

/** Reads the operator's access token, or `null` before login. The token is read per call, never stored here. */
export type TokenProvider = () => Promise<string | null>;

/** What one HTTP GraphQL call needs; every part is injected so that tests need no network (CQ-021). */
export interface HttpTransport {
  readonly endpoint: string;
  readonly fetch: typeof fetch;
  readonly token: TokenProvider;
  readonly signal: (signal: StatusSignal) => void;
}

async function headersFor(token: TokenProvider): Promise<Record<string, string>> {
  const value = await token();
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    accept: 'application/graphql-response+json, application/json',
  };
  if (value !== null) headers.authorization = `Bearer ${value}`;
  return headers;
}

/** The answer text, or `undefined` when no complete answer arrived. */
async function answerText(transport: HttpTransport, body: string): Promise<string | undefined> {
  const headers = await headersFor(transport.token);
  try {
    const response = await transport.fetch(transport.endpoint, { method: 'POST', headers, body });
    return await response.text();
  } catch {
    // fetch rejects only when no answer arrived (network, TLS, reset); the reason is the same for the operator.
    return undefined;
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
 * Resolves with a typed result for every hub or network outcome; rejects only when `token` rejects.
 */
export async function sendRequest<TResult, TVariables>(
  transport: HttpTransport,
  document: TypedDocumentNode<TResult, TVariables>,
  variables: TVariables,
): Promise<HubResult<TResult>> {
  const text = await answerText(transport, JSON.stringify({ query: print(document), variables }));
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
