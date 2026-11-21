import type { RunnerShell } from '$lib/desktop/agent-threads/runner';
import { noticeFor, type Looking } from './notify';
import { REFUSAL_WORDS, type Action, type Outcome, type Update } from './run';
import { applied, begun, type LiveThread } from './timeline';

/**
 * After the watch reaches the runner, the runner sends the updates it keeps again; for this long no notification is
 * sent, so that an old request does not notify a second time (THR-059).
 */
export const REPLAY_MS = 2_000;

/** What a call came to: its outcome, or the refusal in words for the person. */
export type CallResult = { readonly ok: true; readonly outcome: Outcome } | { readonly ok: false; readonly words: string };

/**
 * The threads that the runner of this computer drives, as the page shows them (TH14): the watch folds each update into
 * its thread, and each call goes to the runner. A start over the soft limit waits for the person to raise it (THR-114).
 */
export class LiveThreads {
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- replaced whole at each update, never changed in place
  threads = $state<ReadonlyMap<string, LiveThread>>(new Map());
  /** Whether the watch reaches the runner; `null` before the first answer. */
  linked = $state<boolean | null>(null);
  /** A failure of the watch or of a notification, as fixed text. */
  problem = $state<string | null>(null);
  /** A start that the runner refused at the soft limit: it is sent again when the person raises the limit. */
  overLimit = $state<Action | null>(null);
  private quietUntil = 0;

  constructor(
    private readonly shell: RunnerShell,
    private readonly looking: () => Looking,
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- the clock: read once at each call, never kept
    private readonly now: () => Date = () => new Date(),
  ) {}

  /** Starts the watch, then reads the rows of the threads that have no kept update. Gives the stop. */
  async begin(): Promise<() => void> {
    try {
      const stop = await this.shell.watch(
        (update) => {
          this.take(update);
        },
        (linked) => {
          this.linked = linked;
          if (linked) this.quietUntil = this.now().getTime() + REPLAY_MS;
        },
      );
      await this.seed();
      return stop;
    } catch (reason) {
      this.linked = false;
      this.problem = reason instanceof Error ? reason.message : 'The thread runner did not answer.';
      return () => undefined;
    }
  }

  private async seed(): Promise<void> {
    const listed = await this.shell.call({ call: 'list' });
    if (listed.outcome !== 'threads') return;
    const at = this.now();
    const next = new Map(this.threads);
    for (const row of listed.threads) if (!next.has(row.id)) next.set(row.id, begun(row, at));
    this.threads = next;
  }

  /** One update of the watch; an update of a thread that the page does not know yet waits for its row. */
  take(update: Update): void {
    const at = this.now();
    const before = this.threads.get(update.thread);
    const base = before ?? (update.change.change === 'state' ? begun(update.change.row, at) : undefined);
    if (base === undefined) return;
    const after = applied(base, update, at);
    if (after === before) return;
    this.threads = new Map(this.threads).set(update.thread, after);
    if (at.getTime() < this.quietUntil) return;
    const notice = noticeFor(before, after, update, this.looking());
    if (notice !== null) {
      this.shell.notify(notice.title, notice.words).catch((reason: unknown) => {
        this.problem = reason instanceof Error ? reason.message : 'The notification was not shown.';
      });
    }
  }

  /** One call to the runner. A start over the soft limit is kept for [`raiseLimit`]. */
  async call(action: Action): Promise<CallResult> {
    let outcome: Outcome;
    try {
      outcome = await this.shell.call(action);
    } catch (reason) {
      return { ok: false, words: reason instanceof Error ? reason.message : 'The thread runner did not answer.' };
    }
    if (outcome.outcome !== 'refused') return { ok: true, outcome };
    if (outcome.why === 'over_limit' && action.call === 'start') this.overLimit = action;
    return { ok: false, words: REFUSAL_WORDS[outcome.why] };
  }

  /** The person raised the soft limit: the runner raises it, and the start that waited goes again. */
  async raiseLimit(): Promise<CallResult> {
    const waiting = this.overLimit;
    this.overLimit = null;
    const raised = await this.call({ call: 'raise_limit' });
    if (!raised.ok || waiting === null) return raised;
    return this.call(waiting);
  }

  /** The person did not raise the limit: the start that waited is dropped. */
  keepLimit(): void {
    this.overLimit = null;
  }
}
