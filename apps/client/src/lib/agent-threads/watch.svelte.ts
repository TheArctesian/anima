import { desktopRunnerShell } from '$lib/desktop/agent-threads/runner';
import { desktopThreadsShell } from '$lib/desktop/agent-threads/shell';
import { LiveThreads } from './live/live.svelte';
import { ThreadsStore } from './store.svelte';

/**
 * How often the list is read again while the window is seen (THR-058). The desktop shell has no watcher of the session
 * files yet, so this is a read at an interval, and at each return to the window; a hidden window reads nothing.
 */
export const REFRESH_MS = 30_000;

let shared: ThreadsStore | undefined;

/** The one store of the threads of this computer: the page and the count on the bar read the same rows. */
export function threadsStore(): ThreadsStore {
  if (shared !== undefined) return shared;
  const live = desktopRunnerShell === null ? null : new LiveThreads(desktopRunnerShell, () => ({ front: document.visibilityState === 'visible' && document.hasFocus(), chosen: shared?.chosen ?? null }));
  shared = new ThreadsStore(desktopThreadsShell, typeof localStorage === 'undefined' ? null : localStorage, undefined, live);
  if (live !== null) void live.begin();
  return shared;
}

/**
 * Keeps `store` fresh while something shows it: a read now, one each `REFRESH_MS` while the document is visible, and one
 * when the window is seen again. Returns the stop. Call from an effect.
 */
export function keepFresh(store: ThreadsStore, target: Pick<Document, 'visibilityState' | 'addEventListener' | 'removeEventListener'> = document): () => void {
  const read = (): void => {
    if (target.visibilityState === 'visible') void store.loadList();
  };
  read();
  const timer = setInterval(read, REFRESH_MS);
  target.addEventListener('visibilitychange', read);
  return () => {
    clearInterval(timer);
    target.removeEventListener('visibilitychange', read);
  };
}
