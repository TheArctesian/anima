import { graphql } from '$lib/hub/generated';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

// Each document names the account fields in full: the generated types are per document, and the client keeps no fragments.

const SOURCES = graphql(`
  query Sources {
    sources {
      key
      name
      accounts {
        id
        sourceKey
        label
        instanceKey
        health
        pausedBy
        lastSyncAt
        lastError
        syncing
        runs(first: 20) {
          id
          trigger
          startedAt
          endedAt
          outcome
          inserted
          changed
          unchanged
          removed
          error
        }
      }
    }
  }
`);

const CONNECT_GITHUB_APP = graphql(`
  mutation ConnectGithubApp($input: GithubAppInput!) {
    connectGithubApp(input: $input) {
      id
      sourceKey
      label
      instanceKey
      health
      pausedBy
      lastSyncAt
      lastError
      syncing
      runs(first: 20) {
        id
        trigger
        startedAt
        endedAt
        outcome
        inserted
        changed
        unchanged
        removed
        error
      }
    }
  }
`);

const SYNC_SOURCE_ACCOUNT = graphql(`
  mutation SyncSourceAccount($id: ID!) {
    syncSourceAccount(id: $id) {
      id
      sourceKey
      label
      instanceKey
      health
      pausedBy
      lastSyncAt
      lastError
      syncing
      runs(first: 20) {
        id
        trigger
        startedAt
        endedAt
        outcome
        inserted
        changed
        unchanged
        removed
        error
      }
    }
  }
`);

const SET_SOURCE_ACCOUNT_PAUSED = graphql(`
  mutation SetSourceAccountPaused($id: ID!, $paused: Boolean!) {
    setSourceAccountPaused(id: $id, paused: $paused) {
      id
      sourceKey
      label
      instanceKey
      health
      pausedBy
      lastSyncAt
      lastError
      syncing
      runs(first: 20) {
        id
        trigger
        startedAt
        endedAt
        outcome
        inserted
        changed
        unchanged
        removed
        error
      }
    }
  }
`);

const REMOVE_SOURCE_ACCOUNT = graphql(`
  mutation RemoveSourceAccount($id: ID!, $deleteRecords: Boolean!) {
    removeSourceAccount(id: $id, deleteRecords: $deleteRecords)
  }
`);

const SOURCE_ACCOUNT_CHANGED = graphql(`
  subscription SourceAccountChanged {
    sourceAccountChanged {
      id
      sourceKey
      label
      instanceKey
      health
      pausedBy
      lastSyncAt
      lastError
      syncing
      runs(first: 20) {
        id
        trigger
        startedAt
        endedAt
        outcome
        inserted
        changed
        unchanged
        removed
        error
      }
    }
  }
`);

export type SourceHealth = 'OK' | 'STARTING' | 'STALLED' | 'NEVER_SYNCED' | 'PAUSED' | 'NEEDS_RECONNECT';
export type PauseReason = 'OPERATOR' | 'EXTENSION_OFF';
export type SyncTrigger = 'CONNECT' | 'SCHEDULE' | 'MANUAL';
export type SyncOutcome = 'OK' | 'FAILED' | 'STOPPED';

/** One sync run with its four counts (SRC-005). `outcome` is null while it runs. */
export interface SyncRun {
  readonly id: string;
  readonly trigger: SyncTrigger;
  readonly startedAt: string;
  readonly endedAt: string | null;
  readonly outcome: SyncOutcome | null;
  readonly inserted: number;
  readonly changed: number;
  readonly unchanged: number;
  readonly removed: number;
  readonly error: string | null;
}

/** One source account as the hub lists it (SRC-003, SRC-009). The credential never comes back (KEY-002). */
export interface SourceAccount {
  readonly id: string;
  readonly sourceKey: string;
  readonly label: string;
  readonly instanceKey: string;
  readonly health: SourceHealth;
  readonly pausedBy: PauseReason | null;
  readonly lastSyncAt: string | null;
  readonly lastError: string | null;
  readonly syncing: boolean;
  readonly runs: readonly SyncRun[];
}

/** A source the hub reads, with its accounts. */
export interface Source {
  readonly key: string;
  readonly name: string;
  readonly accounts: readonly SourceAccount[];
}

/** What the operator types to connect a GitHub App (PRJ-018). `privateKey` is a secret: send it, then drop it. */
export interface GithubAppInput {
  readonly label: string;
  readonly appId: string;
  readonly installationId: string;
  readonly privateKey: string;
}

/** The source calls of the hub. A refused call comes back with the hub's own words. */
export interface SourcesApi {
  list(): Promise<HubResult<readonly Source[]>>;
  connectGithubApp(input: GithubAppInput): Promise<HubResult<SourceAccount>>;
  sync(id: string): Promise<HubResult<SourceAccount>>;
  setPaused(id: string, paused: boolean): Promise<HubResult<SourceAccount>>;
  /** `deleteRecords` false keeps the records the account brought in. */
  remove(id: string, deleteRecords: boolean): Promise<HubResult<string>>;
  /** Listens for each change of an account; returns the function that stops listening. */
  watch(sink: Sink<SourceAccount>): () => void;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function sourcesApi(hub: Pick<Hub, 'request' | 'subscribe'>): SourcesApi {
  return {
    list: async () => mapValue(await hub.request(SOURCES, {}), (value) => value.sources),
    connectGithubApp: async (input) => mapValue(await hub.request(CONNECT_GITHUB_APP, { input }), (value) => value.connectGithubApp),
    sync: async (id) => mapValue(await hub.request(SYNC_SOURCE_ACCOUNT, { id }), (value) => value.syncSourceAccount),
    setPaused: async (id, paused) => mapValue(await hub.request(SET_SOURCE_ACCOUNT_PAUSED, { id, paused }), (value) => value.setSourceAccountPaused),
    remove: async (id, deleteRecords) => mapValue(await hub.request(REMOVE_SOURCE_ACCOUNT, { id, deleteRecords }), (value) => value.removeSourceAccount),
    watch: (sink) =>
      hub.subscribe(
        SOURCE_ACCOUNT_CHANGED,
        {},
        {
          next: (value) => {
            sink.next(value.sourceAccountChanged);
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
