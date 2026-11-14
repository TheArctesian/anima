import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { SourceAccount, SyncRun } from './api';

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
  /** Each call sent, as `Operation` and its variables, in order. */
  readonly sent: { readonly operation: string; readonly variables: Record<string, unknown> }[];
  /** What the next mutation answers instead of doing anything. */
  refuse: HubError | null;
  /** Hands an event to every open `sourceAccountChanged` subscription. */
  emit(account: SourceAccount): void;
}

/** A hub with one GitHub source that lists `start`, applies the account calls and sends `sourceAccountChanged`. */
export function fakeHub(start: readonly SourceAccount[]): FakeHub {
  let accounts = [...start];
  const sinks = new Set<{ next(value: unknown): void }>();
  const answer = <T>(value: unknown): Promise<HubResult<T>> => Promise.resolve({ ok: true, value } as HubResult<T>);
  const hub: FakeHub = {
    sent: [],
    refuse: null,
    emit: (changed) => {
      for (const sink of sinks) sink.next({ sourceAccountChanged: changed });
    },
    request: <TResult, TVariables extends Record<string, unknown>>(document: TypedDocumentNode<TResult, TVariables>, variables: TVariables) => {
      const operation = (document.definitions[0] as { name?: { value: string } }).name?.value ?? '';
      if (operation === 'Sources') return answer<TResult>({ sources: [{ key: 'github', name: 'GitHub', accounts }] });
      hub.sent.push({ operation, variables });
      if (hub.refuse !== null) return Promise.resolve({ ok: false, error: hub.refuse } as HubResult<TResult>);
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
    subscribe: (_document, _variables, sink) => {
      sinks.add(sink as { next(value: unknown): void });
      return () => {
        sinks.delete(sink as { next(value: unknown): void });
      };
    },
  };
  return hub;
}
