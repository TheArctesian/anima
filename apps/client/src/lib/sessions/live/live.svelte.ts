import type { Hub } from '$lib/hub/hub';
import { liveSessionsApi, type LiveSession } from './api';
import { applyChange, newestFirst, withoutSession } from './list';

/** More than one operator's sessions at once. */
const LIVE_LIMIT = 200;
const NOT_LOADED = 'The live sessions could not be loaded.';
const NOT_STOPPED = 'The session could not be stopped.';

/**
 * The live sessions as the nav bar shows them (RA-034, D-6): the query's answer, kept current by `sessionChanged`.
 * While the hub does not serve that subscription its error is dropped, and `refresh` is what keeps the list current.
 */
export function watchLiveSessions(hub: Hub) {
  const api = liveSessionsApi(hub);
  let sessions = $state<readonly LiveSession[]>([]);
  let loadFailure = $state<string | null>(null);
  let stopping = $state<readonly string[]>([]);
  let stopFailures = $state<readonly string[]>([]);

  async function refresh(): Promise<void> {
    const result = await api.list(LIVE_LIMIT);
    loadFailure = result.ok ? null : NOT_LOADED;
    if (result.ok) sessions = newestFirst(result.value);
  }

  async function stop(id: string): Promise<void> {
    stopping = [...stopping, id];
    stopFailures = stopFailures.filter((failed) => failed !== id);
    const result = await api.end(id);
    stopping = stopping.filter((busy) => busy !== id);
    if (result.ok) sessions = withoutSession(sessions, id);
    else stopFailures = [...stopFailures, id];
  }

  /** Query once and listen; returns the function that stops listening. */
  function start(): () => void {
    void refresh();
    return api.watch({
      next: (change) => {
        sessions = applyChange(sessions, change);
      },
      // The hub may not serve `sessionChanged` yet: the list keeps the query's answer and never shows this.
      error: () => undefined,
      complete: () => undefined,
    });
  }

  return {
    get sessions() {
      return sessions;
    },
    get loadFailure() {
      return loadFailure;
    },
    stopFailure: (id: string): string | null => (stopFailures.includes(id) ? NOT_STOPPED : null),
    isStopping: (id: string): boolean => stopping.includes(id),
    refresh,
    stop,
    start,
  };
}
