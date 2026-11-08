import type { BrowserContext, WebSocketRoute } from '@playwright/test';
import { parse } from 'graphql';
import { BEARER_TOKEN, callerOf, HUB_ORIGIN, SESSION_COOKIE } from './access';
import type { HubState } from './model';
import type { Recording } from './recorder';

// `wss://hub.anima.ing/graphql`: graphql-transport-ws (API-002). The hub accepts `connection_init` from a socket whose upgrade
// carried the session cookie (K7), or whose payload brings a bearer value, and closes with 4403 otherwise.
// A test page runs on the hub origin, so the upgrade's `Origin` is that origin; Playwright shows no upgrade headers, so the
// cookie is read from the context's jar at the moment of the upgrade.

export const FORBIDDEN_CLOSE = 4403;

interface ClientMessage {
  readonly type: string;
  readonly id?: string;
  readonly payload?: { readonly authorization?: unknown; readonly query?: string; readonly operationName?: string | null };
}

export interface SocketParts {
  readonly context: BrowserContext;
  readonly state: HubState;
  readonly recording: Recording;
}

function operationOf(message: ClientMessage): string | null {
  const query = message.payload?.query;
  if (query === undefined) return null;
  const operation = parse(query).definitions.find((found) => found.kind === 'OperationDefinition');
  return message.payload?.operationName ?? (operation?.kind === 'OperationDefinition' ? (operation.name?.value ?? null) : null);
}

/** Serves one graphql-transport-ws socket. A subscription answers nothing: the fixtures hold no live events. */
export async function serveGraphqlSocket(parts: SocketParts, socket: WebSocketRoute): Promise<void> {
  const jar = await parts.context.cookies(HUB_ORIGIN);
  const cookie = jar.find((found) => found.name === SESSION_COOKIE)?.value ?? null;
  const byCookie = callerOf({ origin: HUB_ORIGIN, authorization: null, cookie }, parts.state.browserSessions) === 'cookie';
  socket.onMessage((raw) => {
    const message = JSON.parse(String(raw)) as ClientMessage;
    const bearer = message.payload?.authorization === `Bearer ${BEARER_TOKEN}`;
    const hadAuthorization = message.payload?.authorization !== undefined;
    switch (message.type) {
      case 'connection_init':
        parts.recording.record({
          channel: 'graphql-socket',
          operation: 'connection_init',
          variables: null,
          hadAuthorization,
          hadCookie: cookie !== null,
          origin: HUB_ORIGIN,
          signedIn: bearer || byCookie,
        });
        if (bearer || byCookie) socket.send(JSON.stringify({ type: 'connection_ack' }));
        else socket.close({ code: FORBIDDEN_CLOSE, reason: 'Forbidden' });
        break;
      case 'ping':
        socket.send(JSON.stringify({ type: 'pong' }));
        break;
      case 'subscribe':
        parts.recording.record({
          channel: 'graphql-socket',
          operation: operationOf(message),
          variables: null,
          hadAuthorization,
          hadCookie: cookie !== null,
          origin: HUB_ORIGIN,
          signedIn: bearer || byCookie,
        });
        break;
      default:
    }
  });
}
