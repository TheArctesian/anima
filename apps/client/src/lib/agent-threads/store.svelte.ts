import type { LiveThreads } from './live/live.svelte';
import { mergedRows } from './live/rows';
import { markSeen, needsYouCount, storedSeen, type Seen } from './state';
import type { Thread, ThreadRow, ThreadsShell } from './thread';

/** The threads of this computer and the one that is open (THR-011). The last answer that was asked for wins. */
export class ThreadsStore {
  rows = $state<readonly ThreadRow[] | null>(null);
  listProblem = $state<string | null>(null);
  thread = $state<Thread | null>(null);
  threadProblem = $state<string | null>(null);
  /** The ID of the thread that is asked for or shown. */
  chosen = $state<string | null>(null);
  loadingThread = $state(false);
  /** When the person last opened each thread: a turn that ended needs the person until the thread is opened (THR-055). */
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- replaced whole at each opening, never changed in place
  seen = $state<Seen>(new Map());
  /** The time of the last read of the list: the states are of this time. */
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- replaced whole at each read, never changed in place
  readAt = $state(new Date());

  constructor(
    private readonly shell: ThreadsShell,
    private readonly storage: Pick<Storage, 'getItem' | 'setItem'> | null = null,
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- the clock: read once at each call, never kept
    private readonly now: () => Date = () => new Date(),
    /** The threads that the runner of this computer drives; `null` where there is no runner. */
    readonly live: LiveThreads | null = null,
  ) {
    if (storage !== null) this.seen = storedSeen(storage);
  }

  /** The rows of the list: the driven threads, then the session files that are not one of them (THR-056). */
  readonly allRows = $derived.by(() => (this.rows === null ? null : mergedRows(this.rows, [...(this.live?.threads.values() ?? [])])));

  /** The driven thread that is open, or `null` when the open thread is a session file. */
  readonly liveThread = $derived.by(() => (this.chosen === null ? null : (this.live?.threads.get(this.chosen) ?? null)));

  /** How many threads need the person (THR-057). */
  readonly needsYou = $derived(needsYouCount(this.allRows ?? [], this.readAt, this.seen));

  /** Reads the list again; the rows on the screen stay until the new ones come. */
  async loadList(): Promise<void> {
    try {
      this.rows = await this.shell.list(null);
      this.readAt = this.now();
      this.listProblem = null;
    } catch (reason) {
      this.listProblem = reason instanceof Error ? reason.message : 'The threads could not be read.';
    }
  }

  /** Opens the thread `id`; an answer for a thread that is no longer chosen is dropped. */
  async open(id: string): Promise<void> {
    this.chosen = id;
    this.seen = markSeen(this.storage, this.seen, id, this.now());
    this.threadProblem = null;
    if (this.live?.threads.has(id) === true) return;
    this.loadingThread = true;
    try {
      const thread = await this.shell.read(id);
      if (this.chosen !== id) return;
      this.thread = thread;
    } catch (reason) {
      if (this.chosen !== id) return;
      this.thread = null;
      this.threadProblem = reason instanceof Error ? reason.message : 'The thread could not be read.';
    } finally {
      if (this.chosen === id) this.loadingThread = false;
    }
  }
}
