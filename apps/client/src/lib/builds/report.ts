import type { BuilderShellCode } from '$lib/desktop/builder/shell';
import type { HubError } from '$lib/hub/hub';
import type { BuildJobState, BuildsApi } from './api';

/** The hub takes at most this many lines in one report (`MAX_LINES_PER_REPORT`). */
export const LINES_PER_REPORT = 1000;
/** Lines waiting for a report are capped; the oldest go first, and the next report says how many went. */
export const PENDING_LINES_MAX = 20_000;

/** What became of one report. `lost`: the hub refused it, so this builder no longer holds the job (BLD-010). */
export type Sent = 'sent' | 'unreachable' | 'lost';

/** A refusal means the lease, the job or the token is gone; anything else may pass on the next try. */
function sentOf(error: HubError): Sent {
  switch (error.code) {
    case 'UNAUTHENTICATED':
    case 'FORBIDDEN':
    case 'NOT_FOUND':
    case 'CONFLICT':
    case 'VALIDATION':
      return 'lost';
    default:
      return 'unreachable';
  }
}

/** The log lines of one job on their way to the hub, and the reports that carry them (BLD-005, BLD-048). */
export class JobReporter {
  #pending: string[] = [];
  /** The number of lines ever queued before `#pending[0]`, sent or dropped. */
  #first = 0;
  #dropped = 0;
  readonly #api: Pick<BuildsApi, 'report'>;
  readonly #jobId: string;

  constructor(api: Pick<BuildsApi, 'report'>, jobId: string) {
    this.#api = api;
    this.#jobId = jobId;
  }

  /** Queues one line that has already been cleaned (BLD-049). */
  add(line: string): void {
    this.#pending.push(line);
    if (this.#pending.length > PENDING_LINES_MAX) {
      this.#pending.shift();
      this.#first += 1;
      this.#dropped += 1;
    }
  }

  /** The next report's lines, and the queue position just after the last of them. */
  #batch(): { lines: string[]; end: number } {
    const note = this.#dropped > 0 ? [`[${String(this.#dropped)} earlier lines were not sent]`] : [];
    const taken = this.#pending.slice(0, LINES_PER_REPORT - note.length);
    return { lines: [...note, ...taken], end: this.#first + taken.length };
  }

  /**
   * Sends every waiting line, `LINES_PER_REPORT` at a time; the last report carries `state` and `failureReason`, so the
   * lines reach the hub while this builder still holds the job (BLD-010). A line leaves the queue only once a report that
   * held it was accepted; lines queued meanwhile wait for the next call.
   */
  async send(state: BuildJobState | null, failureReason: string | null = null): Promise<Sent> {
    const end = this.#first + this.#pending.length;
    for (;;) {
      const batch = this.#batch();
      const last = batch.end >= end;
      const report = { state: last ? state : null, logLines: batch.lines, failureReason: last ? failureReason : null };
      const dropped = this.#dropped;
      const result = await this.#api.report(this.#jobId, report);
      if (!result.ok) return sentOf(result.error);
      this.#pending.splice(0, Math.max(0, batch.end - this.#first));
      this.#first = Math.max(this.#first, batch.end);
      this.#dropped -= Math.min(dropped, this.#dropped);
      if (last) break;
    }
    return 'sent';
  }
}

/** Why a step did not finish: the shell's code, or `hub-refused` when the hub did not take the upload's calls. */
export type StepCode = BuilderShellCode | 'hub-refused';

const REASONS: Partial<Record<StepCode, string>> = {
  'not-on-main': 'The commit is not on main of the Anima repository.',
  dirty: 'The working tree was not clean after the checkout.',
  'tool-missing': 'A build tool is missing on this computer.',
  cancelled: 'The build was cancelled on this computer.',
  bundle: 'The bundle is not where the build puts it.',
  upload: 'A file could not be sent to the release bucket.',
  'hub-refused': 'The hub did not take the upload.',
  busy: 'Another build step was running on this computer.',
  commit: 'The builder refused the job: its commit is not a commit hash.',
  version: 'The builder refused the job: its version is not MAJOR.MINOR.PATCH.',
  target: 'The builder refused the job: its target is unknown.',
  platform: 'The builder refused the job: this computer cannot build its target.',
};

/** The failure reason the hub shows for a step that ended with `code` (BLD-010); fixed text, never process output. */
export function failureReason(stepLabel: string, code: StepCode): string {
  if (code === 'step-failed') return `${stepLabel} failed. The job log says why.`;
  return REASONS[code] ?? `${stepLabel} could not run on this computer.`;
}
