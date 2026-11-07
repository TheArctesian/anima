import type { Hub } from '$lib/hub/hub';
import { liveSessionsApi, type LiveSession } from './api';
import { InFlight } from './inflight';
import { applyChange, newestFirst, withoutSession } from './list';
import { Stopping } from './stopping.svelte';

/** More than one operator's sessions at once. */
const LIVE_LIMIT = 200;
const NOT_LOADED = 'The live sessions could not be loaded.';
/** Shown while `sessionChanged` is in error; the re-query on open keeps the list as current as it can be. */
const LIVE_STOPPED = 'Live updates stopped; the list refreshes when you open it.';

/**
 * The live sessions as the nav bar shows them (RA-034, D-6): the query's answer, kept current by `sessionChanged`.
 * When the subscription fails, `updatesStopped` says so and `refresh` (the query on open) keeps the list current.
 */
export function watchLiveSessions(hub: Hub) {
  const api = liveSessionsApi(hub);
  let sessions = $state<readonly LiveSession[]>([]);
  let loadFailure = $state<string | null>(null);
  let updatesStopped = $state<string | null>(null);
  const stopping = new Stopping();

  const inFlight = new InFlight();

  async function refresh(): Promise<void> {
    const mine = inFlight.begin();
    const result = await api.list(LIVE_LIMIT);
    // A newer refresh has been asked: its answer is the one to show.
    if (!inFlight.isNewest(mine)) return;
    loadFailure = result.ok ? null : NOT_LOADED;
    if (result.ok) sessions = newestFirst(inFlight.catchUp(result.value));
  }

  async function stop(id: string): Promise<void> {
    if (!stopping.begin(id)) return;
    const result = await api.end(id);
    stopping.end(id, result.ok);
    if (result.ok) {
      inFlight.stopped(id);
      sessions = withoutSession(sessions, id);
    }
  }

  /** Query once and listen; returns the function that stops listening. */
  function start(): () => void {
    void refresh();
    return api.watch({
      next: (change) => {
        updatesStopped = null;
        inFlight.pushed(change);
        sessions = applyChange(sessions, change);
      },
      // The list keeps what it has; the text tells the operator it will not change by itself.
      error: () => {
        updatesStopped = LIVE_STOPPED;
      },
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
    get updatesStopped() {
      return updatesStopped;
    },
    stopFailure: stopping.failure,
    isStopping: stopping.isStopping,
    refresh,
    stop,
    start,
  };
}
