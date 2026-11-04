import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { HubAddress } from './address';
import type { HubResult } from './error';
import { sendRequest, type TokenProvider } from './request';
import { createConnection, type LiveStatus } from './status/connection';
import { createSocket, type Sink } from './socket/socket';

export type { HubAddress } from './address';
export { DEFAULT_HUB_ADDRESS, parseHubAddress } from './address';
export type { FieldError, HubCode, HubError, HubResult } from './error';
export type { TokenProvider } from './request';
export type { LiveStatus } from './status/connection';
export type { Connection, HubStatus } from './status/status';
export { dataAge } from './status/age';
export type { Sink } from './socket/socket';

/** GraphQL variables: an object, or an empty one for a document without variables. */
export type Variables = Record<string, unknown>;

/** How long a request waits for its answer, and the socket for `connection_ack`, before the hub counts as unreachable (NET-004). */
const HUB_TIMEOUT_MS = 10_000;

/**
 * The one GraphQL transport of the client (CQ-045). Domain clients in `$lib/<domain>/api.ts` take a `Hub`.
 * Failures are values (`HubResult`), not exceptions: an unreachable hub is an everyday state (NET-004),
 * and the result type makes every caller handle it. Nothing here rejects or throws for a hub, network
 * or token failure; a token provider that rejects gives `UNAUTHENTICATED` ("The access token could not be read.").
 */
export interface Hub {
  readonly address: HubAddress;
  /** Run a query or mutation over HTTP; no answer within the timeout is `unreachable`. */
  request<TResult, TVariables extends Variables>(
    document: TypedDocumentNode<TResult, TVariables>,
    variables: TVariables,
  ): Promise<HubResult<TResult>>;
  /**
   * Start a subscription on the shared socket; returns the function that stops it (API-002).
   * The sink hears `error` (UNAUTHENTICATED) when the hub refuses the token (close 4403) or the token cannot be
   * read; the subscription is not sent again. After a refusal the socket keeps probing on the backoff, reading the
   * token anew each time; after an unreadable token it waits for the next `subscribe`.
   */
  subscribe<TResult, TVariables extends Variables>(
    document: TypedDocumentNode<TResult, TVariables>,
    variables: TVariables,
    sink: Sink<TResult>,
  ): () => void;
  /**
   * Connection status and last contact, as Svelte state (NAV-003, NET-004). The socket keeps it current from
   * creation on: `connected` on `connection_ack` or a refused token, `unreachable` on failure or no ack in time.
   */
  readonly status: LiveStatus;
  /**
   * End every subscription (each sink hears `complete`), stop any connection attempt or retry wait, and close
   * the socket. A later `subscribe` completes at once without connecting; `request` keeps working, since HTTP
   * holds no connection. Subscriptions end synchronously; the promise resolves once closing has begun and
   * rejects with the reason when an open socket could not be closed.
   */
  close(): Promise<void>;
}

export interface HubOptions {
  readonly address: HubAddress;
  readonly token: TokenProvider;
  readonly fetch?: typeof fetch;
  readonly WebSocket?: typeof WebSocket;
  readonly now?: () => Date;
  /** How long to wait for an answer or `connection_ack`; defaults to 10 s (NET-004). */
  readonly timeoutMs?: number;
}

/** A hub at `address`; its socket connects at once, so the status is true before any request (NAV-003, NET-004). */
export function createHub(options: HubOptions): Hub {
  // A frozen copy: a caller that changes its own object later changes neither the endpoints nor what the token provider hears (CLI-002).
  const address: HubAddress = Object.freeze({ ...options.address });
  const { token } = options;
  const timeoutMs = options.timeoutMs ?? HUB_TIMEOUT_MS;
  const connection = createConnection(options.now ?? (() => new Date()));
  const http = {
    address,
    endpoint: address.http,
    fetch: options.fetch ?? globalThis.fetch.bind(globalThis),
    token,
    signal: connection.signal,
    timeoutMs,
  };
  const socket = createSocket({
    address,
    endpoint: address.socket,
    WebSocket: options.WebSocket ?? globalThis.WebSocket,
    token,
    signal: connection.signal,
    timeoutMs,
  });
  return {
    address,
    request: (document, variables) => sendRequest(http, document, variables),
    subscribe: (document, variables, sink) => socket.subscribe(document, variables, sink),
    status: connection.status,
    close: () => socket.close(),
  };
}
