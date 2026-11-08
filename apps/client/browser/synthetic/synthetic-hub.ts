import type { BrowserContext, Route, WebSocketRoute } from '@playwright/test';
import { COOKIE_MAX_AGE_SECONDS, HUB_HOST, HUB_ORIGIN, SESSION_COOKIE } from './access';
import { noteClosing, untilClosed } from './closed';
import { loadFixtures } from './fixtures';
import { answerGraphql } from './graphql/graphql-answers';
import { loadSchema } from './graphql/graphql-schema';
import { newLive } from './graphql/live';
import { serveGraphqlSocket } from './graphql/graphql-socket';
import { newHubState } from './model';
import { newRecording, type Recorder } from './recorder';
import { staticSite, type StaticSite } from './static-files';
import { serveGuacamole, serveTerminal } from './tunnels';

/** A hub that lives inside one browser context. Every request of the context ends here, so no network is used. */
export interface SyntheticHub {
  readonly recording: Recorder;
  /** Gives the context a valid `__Host-anima` cookie, as a good `signInBrowser` would. */
  signIn(): Promise<void>;
  /** Removes the cookie. */
  signOut(): Promise<void>;
  /** Pushes one event to the open subscriptions of `operation` (the document's operation name); returns how many got it. */
  push(operation: string, data: Record<string, unknown>): number;
}

interface Options {
  /** The clock for tickets and cookie expiry; a spec may fix it. */
  readonly now?: () => Date;
}

const METHOD_NOT_ALLOWED = 405;

async function serveRoute(
  route: Route,
  site: StaticSite,
  answer: (route: Route) => Promise<void>,
  refuse: (url: string) => void,
): Promise<void> {
  const request = route.request();
  const url = new URL(request.url());
  if (url.origin !== HUB_ORIGIN) {
    refuse(request.url());
    await route.abort('blockedbyclient');
  } else if (request.method() === 'POST' && url.pathname === '/graphql') {
    await answer(route);
  } else if (url.pathname === '/health' && request.method() === 'GET') {
    await route.fulfill({ status: 200, contentType: 'text/plain', body: 'ok' });
  } else if (request.method() !== 'GET' && request.method() !== 'HEAD') {
    await route.fulfill({ status: METHOD_NOT_ALLOWED, headers: { allow: 'GET, HEAD' }, body: '' });
  } else {
    const answered = site.answer(url.pathname);
    await route.fulfill({ status: answered.status, headers: answered.headers, body: answered.body });
  }
}

type SocketServer = (socket: WebSocketRoute) => void | Promise<void>;

async function serveSocket(socket: WebSocketRoute, serve: Record<string, SocketServer>, refuse: (url: string) => void): Promise<void> {
  const url = new URL(socket.url());
  const serveOne = url.origin === `wss://${HUB_HOST}` ? serve[url.pathname] : undefined;
  if (serveOne === undefined) {
    refuse(socket.url());
    await socket.close({ code: 1008, reason: 'Refused' });
  } else {
    await serveOne(socket);
  }
}

/**
 * Installs the hub on `context`. The page origin is `https://hub.anima.ing`: navigate there. It serves `apps/client/build/`
 * with the K7 headers, runs the K1 schema over the contract fixtures, answers graphql-ws and both tunnels, and aborts and
 * counts every request to another origin. A request still in flight when the test closes its context is dropped quietly.
 */
export async function installSyntheticHub(context: BrowserContext, options: Options = {}): Promise<SyntheticHub> {
  const now = options.now ?? (() => new Date());
  const state = newHubState(loadFixtures(), now);
  const recording = newRecording();
  const live = newLive();
  const schema = loadSchema();
  const site = staticSite();
  const refuse = (url: string): void => recording.refuseForeign(url);
  const graphql = (route: Route): Promise<void> => answerGraphql({ context, schema, state, recording }, route);
  noteClosing(context);
  await context.route('**/*', (route) => untilClosed(() => serveRoute(route, site, graphql, refuse), context));
  const servers: Record<string, SocketServer> = {
    '/graphql': (socket) => serveGraphqlSocket({ context, state, recording, live }, socket),
    '/terminal': (socket) => {
      serveTerminal({ state, recording }, socket);
    },
    '/tunnel': (socket) => {
      serveGuacamole({ state, recording }, socket);
    },
  };
  await context.routeWebSocket(/.*/, (socket) => untilClosed(() => serveSocket(socket, servers, refuse), context));
  return {
    recording,
    async signIn() {
      const expires = Math.floor(now().getTime() / 1000) + COOKIE_MAX_AGE_SECONDS;
      const value = state.browserSessions.issue();
      await context.addCookies([{ name: SESSION_COOKIE, value, url: HUB_ORIGIN, httpOnly: true, secure: true, sameSite: 'Strict', expires }]);
    },
    signOut: () => context.clearCookies({ name: SESSION_COOKIE }),
    push: (operation, data) => live.push(operation, data),
  };
}
