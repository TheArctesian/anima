import { BuilderShellError, type BuilderShell, type BuilderTool, type StepJob, type ToolCheck } from '$lib/desktop/builder/shell';
import type { BuildsApi, ClaimedJob } from './api';
import { LineCleaner } from './redaction';
import { failureReason, JobReporter, type Sent, type StepCode } from './report';
import { uploadRelease, UploadRefusedError } from './upload';

/** How long the builder waits between claims when no job waits; well inside the 60 s that count as online (BLD-007). */
export const POLL_MS = 20_000;
/** How often a running job reports; the hub needs one at least each 15 s (BLD-005). */
export const REPORT_MS = 10_000;
/**
 * The longest a builder call may take: with `REPORT_MS` between reports, a report that the hub accepts still arrives
 * within the 15 s it needs (BLD-005).
 */
export const BUILDER_CALL_TIMEOUT_MS = 4_000;
/** Reports that may fail in a row before the lease (90 s, BLD-005) has surely ended. */
const MISSED_REPORTS_MAX = 8;

/** What the builder is doing, for the Settings row (BLD-020). */
export type BuilderStatus =
  | { readonly kind: 'starting' }
  | { readonly kind: 'missing'; readonly tools: readonly BuilderTool[] }
  /** Every tool is there and the updater key is stored, but it is not unlocked in this app session (D-19). */
  | { readonly kind: 'locked' }
  | { readonly kind: 'unsupported' }
  | { readonly kind: 'tools-unknown' }
  | { readonly kind: 'waiting' }
  | { readonly kind: 'building'; readonly job: ClaimedJob; readonly step: StepName }
  | { readonly kind: 'unreachable' }
  | { readonly kind: 'refused' }
  | { readonly kind: 'stopped' };

export type StepName = 'checkout' | 'gates' | 'bundle' | 'sign' | 'verify' | 'upload';

interface Step {
  readonly name: StepName;
  /** The step's name in a failure reason. */
  readonly label: string;
  run(shell: BuilderShell, job: StepJob): Promise<unknown>;
}

/** The upload, the last step: it runs once the job is reported `UPLOADING` (BLD-060 to BLD-063). */
const UPLOAD = { name: 'upload', label: 'The upload' } as const;

/** The steps of one job, in order (BLD-040 to BLD-046). */
const STEPS: readonly Step[] = [
  { name: 'checkout', label: 'The checkout', run: (shell, job) => shell.checkout(job.commit) },
  { name: 'gates', label: 'The gates', run: (shell, job) => shell.runGates(job.commit) },
  { name: 'bundle', label: 'The bundle', run: (shell, job) => shell.bundle(job) },
  { name: 'sign', label: 'Signing', run: (shell, job) => shell.sign(job) },
  { name: 'verify', label: 'The signature check', run: (shell, job) => shell.verify(job) },
];

/** What the loop needs; every part is injected, time included, so tests run without a shell, a hub or a clock (CQ-021). */
export interface LoopPorts {
  /** The builder's calls, sent with the builder token. */
  readonly api: Pick<BuildsApi, 'claim' | 'report' | 'uploadUrls' | 'reportUploaded'>;
  readonly shell: BuilderShell;
  /** The values removed from every line before it is sent: the builder and operator tokens (BLD-049). */
  readonly secrets: () => readonly string[];
  /** Resolves after `ms`. */
  readonly sleep: (ms: number) => Promise<void>;
  /** The time now; it dates the proposed feed. */
  readonly now: () => Date;
  readonly onStatus: (status: BuilderStatus) => void;
}

type Outcome = { readonly kind: 'done' } | { readonly kind: 'failed'; readonly code: StepCode } | { readonly kind: 'lost' };

function codeOf(reason: unknown): StepCode {
  if (reason instanceof UploadRefusedError) return 'hub-refused';
  return reason instanceof BuilderShellError ? reason.code : 'failed';
}

/**
 * Runs `work` and reports every `REPORT_MS` while it runs, with the lines it printed (BLD-005, BLD-048). When the hub
 * refuses a report, or too many fail in a row, the job is no longer this builder's: the step is cancelled.
 */
async function withReports(work: Promise<unknown>, reporter: JobReporter, ports: LoopPorts): Promise<Outcome> {
  let outcome: Outcome | undefined;
  const settled = work.then(
    (): Outcome => ({ kind: 'done' }),
    (reason: unknown): Outcome => ({ kind: 'failed', code: codeOf(reason) }),
  );
  void settled.then((value) => (outcome = value));
  let missed = 0;
  for (;;) {
    await Promise.race([settled, ports.sleep(REPORT_MS)]);
    if (outcome !== undefined) return outcome;
    const sent: Sent = await reporter.send(null);
    // The work may have ended while the report was out; the upload's last call ends the lease (D-25).
    if (outcome !== undefined) return outcome;
    missed = sent === 'unreachable' ? missed + 1 : 0;
    if (sent === 'lost' || missed >= MISSED_REPORTS_MAX) {
      await ports.shell.cancel().catch(() => undefined);
      await settled;
      return { kind: 'lost' };
    }
  }
}

/** Sends the last report of a job, trying again while the hub cannot be reached and the lease may still hold. */
async function finish(reporter: JobReporter, reason: string, ports: LoopPorts): Promise<void> {
  for (let attempt = 0; attempt < MISSED_REPORTS_MAX; attempt += 1) {
    if ((await reporter.send('FAILED', reason)) !== 'unreachable') return;
    await ports.sleep(REPORT_MS);
  }
}

/** Builds one claimed job, step by step, and reports it (BLD-010, BLD-029). No step starts once `stopping` is true. */
export async function buildJob(job: ClaimedJob, ports: LoopPorts, stopping: () => boolean = () => false): Promise<void> {
  const reporter = new JobReporter(ports.api, job.id);
  const cleaner = new LineCleaner(ports.secrets);
  const stopListening = await ports.shell.onLine((line) => {
    reporter.add(cleaner.clean(line));
  });
  try {
    if ((await reporter.send('BUILDING')) !== 'sent') return;
    for (const step of STEPS) {
      if (stopping()) {
        await finish(reporter, failureReason(step.label, 'cancelled'), ports);
        return;
      }
      ports.onStatus({ kind: 'building', job, step: step.name });
      const outcome = await withReports(step.run(ports.shell, job), reporter, ports);
      if (outcome.kind === 'lost') return;
      if (outcome.kind === 'failed') {
        await finish(reporter, failureReason(step.label, outcome.code), ports);
        return;
      }
    }
    await upload(job, reporter, ports, stopping);
  } finally {
    stopListening();
  }
}

/** Reports the built job `UPLOADING`, then uploads its files and reports them; a failure ends the job (BLD-060 to BLD-063). */
async function upload(job: ClaimedJob, reporter: JobReporter, ports: LoopPorts, stopping: () => boolean): Promise<void> {
  if (stopping()) {
    await finish(reporter, failureReason(UPLOAD.label, 'cancelled'), ports);
    return;
  }
  if ((await reporter.send('UPLOADING')) !== 'sent') return;
  ports.onStatus({ kind: 'building', job, step: UPLOAD.name });
  const outcome = await withReports(uploadRelease(job, ports, stopping), reporter, ports);
  if (outcome.kind === 'failed') await finish(reporter, failureReason(UPLOAD.label, outcome.code), ports);
}

/** The tool check, or `null` after it has said why the builder cannot claim (BLD-028); a locked key claims nothing. */
async function readyTools(ports: LoopPorts): Promise<ToolCheck | null> {
  let tools: ToolCheck;
  try {
    tools = await ports.shell.checkTools();
  } catch (reason) {
    ports.onStatus(codeOf(reason) === 'platform' ? { kind: 'unsupported' } : { kind: 'tools-unknown' });
    return null;
  }
  if (tools.missing.length === 0) return tools;
  const locked = tools.missing.every((tool) => tool === 'updater-key-locked');
  ports.onStatus(locked ? { kind: 'locked' } : { kind: 'missing', tools: tools.missing });
  return null;
}

/** One pass: check the tools, claim, and build what was claimed. Resolves with how long to wait before the next pass. */
export async function builderPass(ports: LoopPorts, stopping: () => boolean = () => false): Promise<number> {
  const tools = await readyTools(ports);
  if (tools === null) return POLL_MS;
  const claimed = await ports.api.claim({ os: tools.os, arch: tools.arch });
  if (!claimed.ok) {
    const refused = claimed.error.code === 'UNAUTHENTICATED' || claimed.error.code === 'FORBIDDEN';
    ports.onStatus({ kind: refused ? 'refused' : 'unreachable' });
    return POLL_MS;
  }
  if (claimed.value === null) {
    ports.onStatus({ kind: 'waiting' });
    return POLL_MS;
  }
  await buildJob(claimed.value, ports, stopping);
  return 0;
}

/**
 * Runs passes until stopped, one job at a time (BLD-029). The returned function stops the loop: it cancels a running
 * step, which ends its processes, and resolves once the loop has finished its last report.
 */
export function startBuilderLoop(ports: LoopPorts): () => Promise<void> {
  let running = true;
  let wake = (): void => undefined;
  const stopped = new Promise<void>((resolve) => {
    wake = resolve;
  });
  const done = (async () => {
    ports.onStatus({ kind: 'starting' });
    while (running) {
      const wait = await builderPass(ports, () => !running);
      if (running && wait > 0) await Promise.race([ports.sleep(wait), stopped]);
    }
    ports.onStatus({ kind: 'stopped' });
  })();
  return async () => {
    running = false;
    wake();
    await ports.shell.cancel().catch(() => undefined);
    await done;
  };
}
