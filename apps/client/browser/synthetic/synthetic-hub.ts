import type { BrowserContext, Route, WebSocketRoute } from '@playwright/test';
import { COOKIE_MAX_AGE_SECONDS, HUB_ORIGIN, SESSION_COOKIE } from './access';
import { loadFixtures } from './fixtures';
import { answerGraphql } from './graphql-answers';
import { loadSchema } from './graphql-schema';
import { serveGraphqlSocket } from './graphql-socket';
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
}

interface Options {
  /** The clock for tickets and expiry; a spec may fix it. */
  readonly now?: () => Date;
}

const METHOD_NOT_ALLOWED = 405;

async function installHttp(
  context: BrowserContext,
  site: StaticSite,
  answer: (route: Route) => Promise<void>,
  refuse: (url: string) => void,
): Promise<void> {
  await context.route('**/*', async (route) => {
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
  });
}

async function installSockets(
  context: BrowserContext,
  serve: Record<string, (socket: WebSocketRoute) => void | Promise<void>>,
  refuse: (url: string) => void,
): Promise<void> {
  await context.routeWebSocket(/.*/, async (socket) => {
    const url = new URL(socket.url());
    const serveOne = url.origin === `wss://${new URL(HUB_ORIGIN).host}` ? serve[url.pathname] : undefined;
    if (serveOne === undefined) {
      refuse(socket.url());
      await socket.close({ code: 1008, reason: 'Refused' });
    } else {
      await serveOne(socket);
    }
  });
}

/**
 * Installs the hub on `context`. The page origin is `https://hub.anima.ing`: navigate there. It serves `apps/client/build/`
 * with the K7 headers, runs the K1 schema over the contract fixtures, answers graphql-ws and both tunnels, and aborts and
 * counts every request to another origin.
 */
export async function installSyntheticHub(context: BrowserContext, options: Options = {}): Promise<SyntheticHub> {
  const state = newHubState(loadFixtures(), options.now ?? (() => new Date()));
  const recording = newRecording();
  const schema = loadSchema();
  const refuse = (url: string): void => recording.refuseForeign(url);
  await installHttp(context, staticSite(), (route) => answerGraphql({ context, schema, state, recording }, route), refuse);
  await installSockets(
    context,
    {
      '/graphql': (socket) => serveGraphqlSocket({ context, state, recording }, socket),
      '/terminal': (socket) => {
        serveTerminal({ state, recording }, socket);
      },
      '/tunnel': (socket) => {
        serveGuacamole({ state, recording }, socket);
      },
    },
    refuse,
  );
  return {
    recording,
    async signIn() {
      const expires = Math.floor(Date.now() / 1000) + COOKIE_MAX_AGE_SECONDS;
      const value = state.browserSessions.issue();
      await context.addCookies([{ name: SESSION_COOKIE, value, url: HUB_ORIGIN, httpOnly: true, secure: true, sameSite: 'Strict', expires }]);
    },
    signOut: () => context.clearCookies({ name: SESSION_COOKIE }),
  };
}
