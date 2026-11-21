import { graphql } from '$lib/hub/generated';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

// The six documents of the three-press GitHub connection (PRJ-039, PRJ-041). `api.ts` is full, so they live here.

/**
 * Whether the hub can start the flow in a browser, in a document of its own: a hub of before the field refuses the whole
 * document, and only this answer then counts as "cannot" (K-E1 Section 7a).
 */
const GITHUB_BROWSER_FLOW = graphql(`
  query GithubBrowserFlow {
    sources {
      key
      connectPageUrl
    }
  }
`);

const GITHUB_CONNECT = graphql(`
  query GithubConnect {
    githubConnect {
      id
      phase
      startedBy
      expiresAt
      pageUrl
      appSlug
      ownerLogin
      installUrl
      appSettingsUrl
      failure
      message
      account {
        health
        pausedBy
      }
    }
  }
`);

const START_GITHUB_CONNECT = graphql(`
  mutation StartGithubConnect($input: StartGithubConnectInput) {
    startGithubConnect(input: $input) {
      id
      phase
      startedBy
      expiresAt
      pageUrl
      appSlug
      ownerLogin
      installUrl
      appSettingsUrl
      failure
      message
      account {
        health
        pausedBy
      }
    }
  }
`);

const OPEN_GITHUB_CONNECT = graphql(`
  mutation OpenGithubConnect($id: ID!) {
    openGithubConnect(id: $id) {
      url
    }
  }
`);

const CANCEL_GITHUB_CONNECT = graphql(`
  mutation CancelGithubConnect($id: ID!) {
    cancelGithubConnect(id: $id) {
      id
      phase
      startedBy
      expiresAt
      pageUrl
      appSlug
      ownerLogin
      installUrl
      appSettingsUrl
      failure
      message
      account {
        health
        pausedBy
      }
    }
  }
`);

const GITHUB_CONNECT_CHANGED = graphql(`
  subscription GithubConnectChanged {
    githubConnectChanged {
      id
      phase
      startedBy
      expiresAt
      pageUrl
      appSlug
      ownerLogin
      installUrl
      appSettingsUrl
      failure
      message
      account {
        health
        pausedBy
      }
    }
  }
`);

export type ConnectPhase = 'WAITING_FOR_BROWSER' | 'AT_GITHUB' | 'APP_CREATED' | 'CONNECTED' | 'FAILED' | 'CANCELLED';
export type ConnectFailure = 'EXPIRED' | 'EXCHANGE_REFUSED' | 'GITHUB_UNREACHABLE' | 'REPLY_UNUSABLE' | 'KEY_REFUSED';

/** The attempt the hub holds. It has no key, code, ticket or state (PRJ-041). Text from GitHub is `appSlug` and `ownerLogin`. */
export interface ConnectAttempt {
  readonly id: string;
  readonly phase: ConnectPhase;
  readonly startedBy: 'USER' | 'AGENT' | 'HUB';
  readonly expiresAt: string;
  /** The hub page a signed-in operator opens. Holds no one-time value. */
  readonly pageUrl: string;
  readonly appSlug: string | null;
  readonly ownerLogin: string | null;
  readonly installUrl: string | null;
  readonly appSettingsUrl: string | null;
  readonly failure: ConnectFailure | null;
  readonly message: string | null;
  /** The new account, from CONNECTED on: only what the state line needs. */
  readonly account: { readonly health: string; readonly pausedBy: string | null } | null;
}

/** The calls of the connection. The one-time link is returned to the caller once and kept nowhere. */
export interface GithubConnectApi {
  /**
   * The hub page where a browser continues the flow, or null when the hub cannot start it in a browser: it says so, it is
   * a hub of before the field, or it did not answer.
   */
  browserPage(): Promise<string | null>;
  current(): Promise<HubResult<ConnectAttempt | null>>;
  /** The personal account only: the client sends no organisation. */
  start(): Promise<HubResult<ConnectAttempt>>;
  /** ONE-TIME: a new link; an earlier one stops working. */
  link(id: string): Promise<HubResult<string>>;
  cancel(id: string): Promise<HubResult<ConnectAttempt>>;
  watch(sink: Sink<ConnectAttempt>): () => void;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function githubConnectApi(hub: Pick<Hub, 'request' | 'subscribe'>): GithubConnectApi {
  return {
    browserPage: async () => {
      const result = await hub.request(GITHUB_BROWSER_FLOW, {});
      if (!result.ok) return null;
      return result.value.sources.find((source) => source.key === 'github')?.connectPageUrl ?? null;
    },
    current: async () => mapValue(await hub.request(GITHUB_CONNECT, {}), (value) => value.githubConnect),
    start: async () => mapValue(await hub.request(START_GITHUB_CONNECT, { input: {} }), (value) => value.startGithubConnect),
    link: async (id) => mapValue(await hub.request(OPEN_GITHUB_CONNECT, { id }), (value) => value.openGithubConnect.url),
    cancel: async (id) => mapValue(await hub.request(CANCEL_GITHUB_CONNECT, { id }), (value) => value.cancelGithubConnect),
    watch: (sink) =>
      hub.subscribe(
        GITHUB_CONNECT_CHANGED,
        {},
        {
          next: (value) => {
            sink.next(value.githubConnectChanged);
          },
          error: (error) => {
            sink.error(error);
          },
          complete: () => {
            sink.complete();
          },
        },
      ),
  };
}
