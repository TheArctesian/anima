import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { SourceAccount, SyncRun } from './api';
import type { ConnectAttempt } from './connect/api';
import { attempt, connectAnswer } from './connect/connect.fixture';

export { attempt };

// Test support: source accounts in the hub's shape and a hub that answers the source calls from memory. The hub does not
// serve these fields yet (K-E1 is pending), so the section's tests run against this one.

export function run(id: string, more: Partial<SyncRun> = {}): SyncRun {
  return {
    id,
    trigger: 'SCHEDULE',
    startedAt: '2026-11-13T11:50:00Z',
    endedAt: '2026-11-13T11:50:04Z',
    outcome: 'OK',
    inserted: 0,
    changed: 0,
    unchanged: 0,
    removed: 0,
    error: null,
    ...more,
  };
}

export function account(id: string, label: string, more: Partial<SourceAccount> = {}): SourceAccount {
  return {
    id,
    sourceKey: 'github',
    label,
    instanceKey: '1/2',
    health: 'OK',
    pausedBy: null,
    lastSyncAt: '2026-11-13T11:56:00Z',
    lastError: null,
    syncing: false,
    runs: [],
    ...more,
  };
}

export const NOW = new Date('2026-11-13T12:00:00Z');

export interface FakeHub extends Pick<Hub, 'request' | 'subscribe'> {
  /**
   * What `Source.connectPageUrl` of GitHub answers: a page, null for a hub that cannot start the flow in a browser, or
   * undefined for a hub of before the field, which refuses the document as a hub refuses an unknown field.
   */
  connectPage: string | null | undefined;
  /** Each call sent, as `Operation` and its variables, in order. */
  readonly sent: { readonly operation: string; readonly variables: Record<string, unknown> }[];
  /** What the next mutation answers instead of doing anything. */
  refuse: HubError | null;
  /** Hands an event to every open `sourceAccountChanged` subscription. */
  emit(account: SourceAccount): void;
  /** The GitHub connection the hub holds; `githubConnect` answers it and `startGithubConnect` sets it. */
  connect: ConnectAttempt | null;
  /** The one-time link `openGithubConnect` answers. */
  link: string;
  /** Hands an attempt to every open `githubConnectChanged` subscription. */
  emitAttempt(attempt: ConnectAttempt): void;
}

/** The answer to `GithubBrowserFlow`; a hub of before the field refuses the document. */
function browserFlow<T>(page: string | null | undefined): Promise<HubResult<T>> {
  if (page === undefined) {
    const refused: HubResult<T> = { ok: false, error: { code: 'INTERNAL', message: 'The hub could not run the request.' } };
    return Promise.resolve(refused);
  }
  const value = { sources: [{ key: 'github', connectPageUrl: page }] };
  return Promise.resolve({ ok: true, value } as HubResult<T>);
}

/** A hub with one GitHub source that lists `start`, applies the account calls and sends `sourceAccountChanged`. */
export function fakeHub(start: readonly SourceAccount[]): FakeHub {
  let accounts = [...start];
  const sinks = new Set<{ next(value: unknown): void }>();
  const attemptSinks = new Set<{ next(value: unknown): void }>();
  const answer = <T>(value: unknown): Promise<HubResult<T>> => Promise.resolve({ ok: true, value } as HubResult<T>);
  const hub: FakeHub = {
    connectPage: 'https://hub.example/connect/github',
    sent: [],
    refuse: null,
    connect: null,
    link: 'https://hub.example/connect/github?ticket=ONE-TIME-TICKET',
    emitAttempt: (changed) => {
      hub.connect = changed;
      for (const sink of attemptSinks) sink.next({ githubConnectChanged: changed });
    },
    emit: (changed) => {
      for (const sink of sinks) sink.next({ sourceAccountChanged: changed });
    },
    request: <TResult, TVariables extends Record<string, unknown>>(document: TypedDocumentNode<TResult, TVariables>, variables: TVariables) => {
      const operation = (document.definitions[0] as { name?: { value: string } }).name?.value ?? '';
      if (operation === 'Sources') return answer<TResult>({ sources: [{ key: 'github', name: 'GitHub', accounts }] });
      if (operation === 'GithubConnect') return answer<TResult>({ githubConnect: hub.connect });
      if (operation === 'GithubBrowserFlow') return browserFlow<TResult>(hub.connectPage);
      hub.sent.push({ operation, variables });
      if (hub.refuse !== null) return Promise.resolve({ ok: false, error: hub.refuse } as HubResult<TResult>);
      const connecting = connectAnswer(hub, operation);
      if (connecting !== undefined) return answer<TResult>(connecting);
      if (operation === 'ConnectGithubApp') {
        const made = account('new', String((variables['input'] as { label: string }).label), { health: 'STARTING', lastSyncAt: null });
        accounts = [...accounts, made];
        return answer<TResult>({ connectGithubApp: made });
      }
      const id = String(variables['id']);
      if (operation === 'RemoveSourceAccount') {
        accounts = accounts.filter((each) => each.id !== id);
        return answer<TResult>({ removeSourceAccount: id });
      }
      const found = accounts.find((each) => each.id === id);
      if (found === undefined) return Promise.resolve({ ok: false, error: { code: 'NOT_FOUND', message: 'No such account.' } } as HubResult<TResult>);
      const changed: SourceAccount =
        operation === 'SetSourceAccountPaused'
          ? { ...found, health: variables['paused'] === true ? 'PAUSED' : 'OK', pausedBy: variables['paused'] === true ? 'OPERATOR' : null }
          : { ...found, syncing: true };
      accounts = accounts.map((each) => (each.id === id ? changed : each));
      return answer<TResult>({ [operation === 'SyncSourceAccount' ? 'syncSourceAccount' : 'setSourceAccountPaused']: changed });
    },
    subscribe: (document, _variables, sink) => {
      const operation = (document.definitions[0] as { name?: { value: string } }).name?.value;
      const open = operation === 'GithubConnectChanged' ? attemptSinks : sinks;
      open.add(sink as { next(value: unknown): void });
      return () => {
        open.delete(sink as { next(value: unknown): void });
      };
    },
  };
  return hub;
}
