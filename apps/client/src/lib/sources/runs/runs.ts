import type { SyncOutcome, SyncRun, SyncTrigger } from '../api';
import { agoText } from '../health';

const TRIGGER: Record<SyncTrigger, string> = { CONNECT: 'On connect', SCHEDULE: 'On schedule', MANUAL: 'By hand' };
const OUTCOME: Record<SyncOutcome, string> = { OK: 'Done', FAILED: 'Failed', STOPPED: 'Stopped' };

/** One run as lines of words: when it began, how it went, and the four counts (SRC-005). */
export interface RunLine {
  readonly id: string;
  readonly trigger: string;
  readonly started: string;
  readonly ended: string;
  readonly outcome: string;
  readonly counts: readonly { readonly label: string; readonly value: number }[];
  readonly error: string | null;
}

export function runLine(run: SyncRun, now: Date): RunLine {
  return {
    id: run.id,
    trigger: TRIGGER[run.trigger],
    started: `Started ${agoText(run.startedAt, now)}`,
    ended: run.endedAt === null ? 'Not ended' : `Ended ${agoText(run.endedAt, now)}`,
    outcome: run.outcome === null ? 'Running' : OUTCOME[run.outcome],
    counts: [
      { label: 'Inserted', value: run.inserted },
      { label: 'Changed', value: run.changed },
      { label: 'Unchanged', value: run.unchanged },
      { label: 'Removed', value: run.removed },
    ],
    error: run.error,
  };
}
