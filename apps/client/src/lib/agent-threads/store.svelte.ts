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

  constructor(private readonly shell: ThreadsShell) {}

  /** Reads the list again; the rows on the screen stay until the new ones come. */
  async loadList(): Promise<void> {
    try {
      this.rows = await this.shell.list(null);
      this.listProblem = null;
    } catch (reason) {
      this.listProblem = reason instanceof Error ? reason.message : 'The threads could not be read.';
    }
  }

  /** Opens the thread `id`; an answer for a thread that is no longer chosen is dropped. */
  async open(id: string): Promise<void> {
    this.chosen = id;
    this.loadingThread = true;
    this.threadProblem = null;
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
