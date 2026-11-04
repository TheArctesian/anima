import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { createClient, type Client } from 'graphql-ws';
import { MALFORMED, parseErrors, parseResponse, UNREACHABLE, type HubError } from './error';
import type { TokenProvider } from './request';
import type { StatusSignal } from './status/status';

/** Receives one subscription's events. After `error` or `complete`, nothing more arrives. */
export interface Sink<T> {
  next(value: T): void;
  error(error: HubError): void;
  complete(): void;
}

/** What the subscription socket needs; every part is injected so that tests need no network (CQ-021). */
export interface SocketTransport {
  readonly endpoint: string;
  readonly WebSocket: typeof WebSocket;
  readonly token: TokenProvider;
  readonly signal: (signal: StatusSignal) => void;
}

/** The hub closes with 4403 when the token is missing or invalid (graphql-transport-ws `Forbidden`). */
export const REFUSED_CLOSE_CODE = 4403;
const NORMAL_CLOSE_CODE = 1000;
/** graphql-transport-ws `InternalServerError` and `InternalClientError`. */
const INTERNAL_CLOSE_CODES = [4500, 4005];
const FIRST_RETRY_MS = 1000;
const LONGEST_RETRY_MS = 30_000;

/** Wait before reconnect attempt `retries` (0-based): doubling from 1 s, at most 30 s, so retries never stop. */
export function retryDelayMs(retries: number): number {
  return Math.min(FIRST_RETRY_MS * 2 ** retries, LONGEST_RETRY_MS);
}

function closeCode(value: unknown): number | undefined {
  if (typeof value !== 'object' || value === null || !('code' in value)) return undefined;
  return typeof value.code === 'number' ? value.code : undefined;
}

/** The status signal for a socket close, or `undefined` for a normal close (the last subscription ended). */
export function closeSignal(event: unknown): StatusSignal | undefined {
  const code = closeCode(event);
  if (code === NORMAL_CLOSE_CODE) return undefined;
  return code === REFUSED_CLOSE_CODE ? 'refused' : 'closed';
}

/** Why a subscription ended without `complete`. */
export function subscriptionError(reason: unknown): HubError {
  if (Array.isArray(reason)) return parseErrors(reason);
  const code = closeCode(reason);
  if (code === undefined) return UNREACHABLE;
  if (code === REFUSED_CLOSE_CODE) return { code: 'UNAUTHENTICATED', message: 'The hub refused the access token. Log in again.' };
  if (INTERNAL_CLOSE_CODES.includes(code)) return { code: 'INTERNAL', message: 'The live connection to the hub failed.' };
  return code >= 4000 && code <= 4999 ? MALFORMED : UNREACHABLE;
}

/** The single graphql-transport-ws client of one hub (API-002); it opens lazily and retries until the hub answers. */
export function createSocket(transport: SocketTransport): Client {
  return createClient({
    url: transport.endpoint,
    webSocketImpl: transport.WebSocket,
    // Read per connection, so a new login applies at the next reconnect; never a query parameter (LOG-007).
    connectionParams: async () => {
      const token = await transport.token();
      return token === null ? {} : { authorization: `Bearer ${token}` };
    },
    retryAttempts: Number.POSITIVE_INFINITY,
    retryWait: (retries) => new Promise((resolve) => setTimeout(resolve, retryDelayMs(retries))),
    // Retry every failure except a refused token: retrying with the same token cannot succeed.
    shouldRetry: (reason) => closeCode(reason) !== REFUSED_CLOSE_CODE,
    on: {
      opened: () => transport.signal('opened'),
      error: () => transport.signal('error'),
      closed: (event) => {
        const signal = closeSignal(event);
        if (signal !== undefined) transport.signal(signal);
      },
    },
  });
}

/** Start `document` on `socket` and deliver its events to `sink`; returns the function that stops it. */
export function startSubscription<TResult, TVariables extends Record<string, unknown>>(
  socket: Client,
  document: TypedDocumentNode<TResult, TVariables>,
  variables: TVariables,
  sink: Sink<TResult>,
): () => void {
  let ended = false;
  const end = (deliver: () => void): void => {
    if (ended) return;
    ended = true;
    deliver();
  };
  const stop = socket.subscribe(
    { query: print(document), variables },
    {
      next: (payload) => {
        if (ended) return;
        const result = parseResponse(payload);
        // The document was checked against the schema at build time (API-008); the hub answers in that shape.
        if (result.ok) sink.next(result.value as TResult);
        else end(() => {
          stop();
          sink.error(result.error);
        });
      },
      error: (reason) => end(() => sink.error(subscriptionError(reason))),
      complete: () => end(() => sink.complete()),
    },
  );
  return () => end(stop);
}
