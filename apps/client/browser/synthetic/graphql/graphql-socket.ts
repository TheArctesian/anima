import type { BrowserContext, WebSocketRoute } from '@playwright/test';
import { parse } from 'graphql';
import { callerOf, HUB_ORIGIN, SESSION_COOKIE } from '../access';
import { untilClosed } from '../closed';
import type { HubState } from '../model';
import type { Recording } from '../recorder';
import type { Live } from './live';

// `wss://hub.anima.ing/graphql`: graphql-transport-ws (API-002). The hub accepts `connection_init` from a socket whose upgrade
// carried the session cookie and whose payload has no `authorization` (K7), or whose payload brings the good bearer value,
// and closes with 4403 otherwise.
// Limits of Playwright: it shows no upgrade headers, so the cookie is read from the context's jar when `connection_init`
// arrives, and the upgrade's `Origin` cannot be checked: the socket is taken to come from the hub origin, where every
// test page runs. The cookie's `Origin` rule is proved for HTTP and in the pure `callerOf` tests. Duplicate headers
// (two `Origin` lines) cannot be expressed through Playwright either.

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
  readonly live: Live;
}

function operationOf(message: ClientMessage): string | null {
  const query = message.payload?.query;
  if (query === undefined) return null;
  const operation = parse(query).definitions.find((found) => found.kind === 'OperationDefinition');
  return message.payload?.operationName ?? (operation?.kind === 'OperationDefinition' ? (operation.name?.value ?? null) : null);
}

interface SocketFacts {
  readonly hadAuthorization: boolean;
  readonly hadCookie: boolean;
  readonly signedIn: boolean;
}

function recordSocket(parts: SocketParts, operation: string | null, facts: SocketFacts): void {
  parts.recording.record({ channel: 'graphql-socket', operation, variables: null, origin: HUB_ORIGIN, ...facts });
}

async function factsOf(parts: SocketParts, message: ClientMessage): Promise<SocketFacts> {
  const jar = await parts.context.cookies(HUB_ORIGIN);
  const cookie = jar.find((found) => found.name === SESSION_COOKIE)?.value ?? null;
  const authorization = typeof message.payload?.authorization === 'string' ? message.payload.authorization : null;
  const caller = callerOf({ origin: HUB_ORIGIN, authorization, cookie }, parts.state.browserSessions);
  return { hadAuthorization: message.payload?.authorization !== undefined, hadCookie: cookie !== null, signedIn: caller !== 'signed-out' };
}

/** Serves one graphql-transport-ws socket. A subscription answers nothing until a spec pushes an event through `live`. */
export function serveGraphqlSocket(parts: SocketParts, socket: WebSocketRoute): void {
  let facts: SocketFacts | undefined;
  socket.onMessage((raw) => {
    const message = JSON.parse(String(raw)) as ClientMessage;
    if (message.type === 'ping') socket.send(JSON.stringify({ type: 'pong' }));
    else if (message.type === 'subscribe' && facts !== undefined) {
      const operation = operationOf(message);
      const leave = parts.live.join({
        operation,
        deliver: (data) => socket.send(JSON.stringify({ type: 'next', id: message.id, payload: { data } })),
      });
      socket.onClose(leave);
      recordSocket(parts, operation, facts);
    } else if (message.type === 'connection_init') {
      // The callback is not awaited, so a context closed meanwhile is dropped here.
      void untilClosed(async () => {
        facts = await factsOf(parts, message);
        recordSocket(parts, 'connection_init', facts);
        if (facts.signedIn) socket.send(JSON.stringify({ type: 'connection_ack' }));
        else await socket.close({ code: FORBIDDEN_CLOSE, reason: 'Forbidden' });
      }, parts.context);
    }
  });
}
