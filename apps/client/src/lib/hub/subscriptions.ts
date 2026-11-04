import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { createClient, type Client, type Sink as ClientSink } from 'graphql-ws';
import { MALFORMED, parseErrors, parseResponse, TOKEN_UNREADABLE, UNREACHABLE, type HubError } from './error';
import { readToken, type TokenProvider } from './request';
import type { StatusSignal } from './status/status';

/** Receives one subscription's events. After `error` or `complete`, nothing more arrives. */
export interface Sink<T> {
  next(value: T): void;
  error(error: HubError): void;
  complete(): void;
}

/** What the subscription socket needs; every part is injected so that tests need no network (CQ-021). */
interface SocketTransport {
  readonly endpoint: string;
  readonly WebSocket: typeof WebSocket;
  readonly token: TokenProvider;
  readonly signal: (signal: StatusSignal) => void;
  /** How long to wait for `connection_ack` after the socket opens before closing and retrying (NET-004). */
  readonly timeoutMs: number;
}

/** The one subscription socket of a hub. */
export interface Socket {
  subscribe<TResult, TVariables extends Record<string, unknown>>(
    document: TypedDocumentNode<TResult, TVariables>,
    variables: TVariables,
    sink: Sink<TResult>,
  ): () => void;
  /** End every subscription with `complete` and close the socket; later subscriptions complete at once. */
  close(): void;
}

/** The hub closes with 4403 when the token is missing or invalid (graphql-transport-ws `Forbidden`). */
const REFUSED_CLOSE_CODE = 4403;
const NORMAL_CLOSE_CODE = 1000;
/** graphql-transport-ws `InternalClientError`: the client closed its own socket. */
const CLIENT_CLOSE_CODE = 4005;
const SERVER_ERROR_CLOSE_CODE = 4500;
const FIRST_RETRY_MS = 1000;
const LONGEST_RETRY_MS = 30_000;

/** Wait before reconnect attempt `retries` (0-based): doubling from 1 s, at most 30 s, so retries never stop. */
function retryDelayMs(retries: number): number {
  return Math.min(FIRST_RETRY_MS * 2 ** retries, LONGEST_RETRY_MS);
}

function closeCode(value: unknown): number | undefined {
  if (typeof value !== 'object' || value === null || !('code' in value)) return undefined;
  return typeof value.code === 'number' ? value.code : undefined;
}

/** The status signal for a socket close; none for a normal close or one the client made itself. */
function closeSignal(event: unknown): StatusSignal | undefined {
  const code = closeCode(event);
  if (code === NORMAL_CLOSE_CODE || code === CLIENT_CLOSE_CODE) return undefined;
  return code === REFUSED_CLOSE_CODE ? 'refused' : 'closed';
}

/** Why a subscription ended without `complete`. */
function subscriptionError(reason: unknown, tokenUnreadable: boolean): HubError {
  if (tokenUnreadable) return TOKEN_UNREADABLE;
  if (Array.isArray(reason)) return parseErrors(reason);
  const code = closeCode(reason);
  if (code === undefined) return UNREACHABLE;
  if (code === REFUSED_CLOSE_CODE) return { code: 'UNAUTHENTICATED', message: 'The hub refused the access token. Log in again.' };
  if (code === SERVER_ERROR_CLOSE_CODE || code === CLIENT_CLOSE_CODE) {
    return { code: 'INTERNAL', message: 'The live connection to the hub failed.' };
  }
  return code >= 4000 && code <= 4999 ? MALFORMED : UNREACHABLE;
}

/** Per-socket state that graphql-ws callbacks share: whether the current connection could not read the token. */
interface TokenState {
  unreadable: boolean;
}

function connect(transport: SocketTransport, token: TokenState): Client {
  return createClient({
    url: transport.endpoint,
    webSocketImpl: transport.WebSocket,
    // Read per connection, so a new login applies at the next reconnect; never a query parameter (LOG-007).
    connectionParams: async () => {
      const read = await readToken(transport.token);
      if (!read.ok) {
        token.unreadable = true;
        // graphql-ws closes with 4005; the message is ours, never the provider's.
        throw new Error(read.error.message);
      }
      return read.value === null ? {} : { authorization: `Bearer ${read.value}` };
    },
    connectionAckWaitTimeout: transport.timeoutMs,
    retryAttempts: Number.POSITIVE_INFINITY,
    retryWait: (retries) => new Promise((resolve) => setTimeout(resolve, retryDelayMs(retries))),
    // Retry every failure except a refused or unreadable token: retrying cannot succeed.
    shouldRetry: (reason) => !token.unreadable && closeCode(reason) !== REFUSED_CLOSE_CODE,
    on: {
      connecting: () => {
        token.unreadable = false;
      },
      connected: () => transport.signal('acknowledged'),
      closed: (event) => {
        const signal = closeSignal(event);
        if (signal !== undefined) transport.signal(signal);
      },
    },
  });
}

/** The graphql-ws side of one subscription: parse payloads and map errors. */
function deliverTo<T>(sink: Sink<T>, tokenUnreadable: () => boolean): ClientSink {
  return {
    next: (payload) => {
      const result = parseResponse(payload);
      // The document was checked against the schema at build time (API-008); the hub answers in that shape.
      if (result.ok) sink.next(result.value as T);
      else sink.error(result.error);
    },
    error: (reason) => sink.error(subscriptionError(reason, tokenUnreadable())),
    complete: () => sink.complete(),
  };
}

/** The single graphql-transport-ws socket of one hub (API-002); it opens lazily and retries until the hub answers. */
export function createSocket(transport: SocketTransport): Socket {
  const token: TokenState = { unreadable: false };
  let closed = false;
  const live = new Set<() => void>();
  const client = connect(transport, token);
  return {
    subscribe(document, variables, sink) {
      if (closed) {
        sink.complete();
        return () => undefined;
      }
      let ended = false;
      let stop = (): void => undefined;
      const end = (deliver: () => void): void => {
        if (ended) return;
        ended = true;
        live.delete(finish);
        stop();
        deliver();
      };
      const finish = (): void => end(() => sink.complete());
      const guarded: typeof sink = {
        next: (value) => {
          if (!ended) sink.next(value);
        },
        error: (error) => end(() => sink.error(error)),
        complete: finish,
      };
      live.add(finish);
      stop = client.subscribe({ query: print(document), variables }, deliverTo(guarded, () => token.unreadable));
      return () => end(() => undefined);
    },
    close() {
      closed = true;
      for (const finish of [...live]) finish();
      void client.dispose();
    },
  };
}
