import type { JobView } from '../api';

/**
 * The stages a job passes through, in order. Checkout, gates and bundle (with notarizing on a Mac) are one builder
 * report, `BUILDING`, so they are one stage; the hub signs and publishes once the upload is in (BLD-040 to BLD-044,
 * BLD-060 to BLD-063, R-117).
 */
export type StageName = 'claim' | 'build' | 'upload' | 'publish';

/** `skipped`: the job ended before this stage, or nothing says whether it ran. */
export type StageMark = 'done' | 'current' | 'failed' | 'pending' | 'skipped';

export interface Stage {
  readonly name: StageName;
  readonly label: string;
  readonly mark: StageMark;
  /** What the stage is made of, or where it failed: "checkout, gates, bundle", "the gates failed". */
  readonly detail: string;
}

/** How a state reads: under way, finished well, failed, waiting for a builder, or stopped by the operator. */
export type StateTone = 'active' | 'done' | 'failed' | 'waiting' | 'stopped';

const ORDER: readonly StageName[] = ['claim', 'build', 'upload', 'publish'];

const LABELS: Record<StageName, string> = { claim: 'Claim', build: 'Build', upload: 'Upload', publish: 'Publish' };

/** The step names a builder's failure reason starts with (`failureReason` in the builder loop) and the stage of each. */
const FAILED_STEPS: readonly { readonly prefix: string; readonly stage: StageName; readonly step: string }[] = [
  { prefix: 'The checkout', stage: 'build', step: 'checkout' },
  { prefix: 'The gates', stage: 'build', step: 'gates' },
  { prefix: 'The bundle', stage: 'build', step: 'bundle' },
  { prefix: 'The upload', stage: 'upload', step: 'upload' },
];

/** The stage each live state is in; `PUBLISHED` has none left. */
const CURRENT: Partial<Record<JobView['state'], StageName>> = {
  QUEUED: 'claim',
  CLAIMED: 'build',
  BUILDING: 'build',
  UPLOADING: 'upload',
};

const TONES: Record<JobView['state'], StateTone> = {
  QUEUED: 'waiting',
  CLAIMED: 'active',
  BUILDING: 'active',
  UPLOADING: 'active',
  PUBLISHED: 'done',
  FAILED: 'failed',
  CANCELLED: 'stopped',
};

export function stateTone(state: JobView['state']): StateTone {
  return TONES[state];
}

function buildSteps(target: JobView['target']): string {
  return target === 'DARWIN_UNIVERSAL' ? 'checkout, gates, bundle, notarize' : 'checkout, gates, bundle';
}

function stage(name: StageName, mark: StageMark, detail = ''): Stage {
  return { name, label: LABELS[name], mark, detail };
}

/** Marks every stage before `at` done, `at` with `mark`, and every stage after it `after`. */
function marked(at: StageName, mark: StageMark, after: StageMark, detail: string): Stage[] {
  const index = ORDER.indexOf(at);
  return ORDER.map((name, place) => {
    if (place < index) return stage(name, 'done');
    if (place === index) return stage(name, mark, detail);
    return stage(name, after);
  });
}

/** The failed stage named by a builder's failure reason, or `null` for a reason that names none (a watchdog's). */
function failedAt(reason: string | null): (typeof FAILED_STEPS)[number] | null {
  return FAILED_STEPS.find((known) => reason?.startsWith(known.prefix) === true) ?? null;
}

/**
 * Where `job` has got to, stage by stage, from its state and, for a failure, the step its reason names (BLD-080). A
 * failure whose reason names no step marks only the claim, and only when a builder held the job.
 */
export function jobStages(job: Pick<JobView, 'state' | 'target' | 'failureReason' | 'builderName'>): Stage[] {
  const current = CURRENT[job.state];
  if (current !== undefined) return marked(current, 'current', 'pending', current === 'build' ? buildSteps(job.target) : '');
  if (job.state === 'PUBLISHED') return ORDER.map((name) => stage(name, 'done'));
  if (job.state === 'CANCELLED') return ORDER.map((name) => stage(name, 'skipped'));
  const failed = failedAt(job.failureReason);
  if (failed !== null) return marked(failed.stage, 'failed', 'skipped', `the ${failed.step} failed`);
  return ORDER.map((name) => stage(name, name === 'claim' && job.builderName !== null ? 'done' : 'skipped'));
}

/** What a release's jobs add up to, for its header: one word, its tone, and how many targets published. */
export interface ReleaseSummary {
  readonly word: string;
  readonly tone: StateTone;
  readonly published: string;
}

function summaryWord(jobs: readonly Pick<JobView, 'state'>[]): { word: string; tone: StateTone } {
  const has = (state: JobView['state']) => jobs.some((job) => job.state === state);
  if (jobs.some((job) => stateTone(job.state) === 'active')) return { word: 'Building', tone: 'active' };
  if (has('QUEUED')) return { word: 'Queued', tone: 'waiting' };
  if (has('FAILED')) return { word: 'Failed', tone: 'failed' };
  if (jobs.length > 0 && jobs.every((job) => job.state === 'PUBLISHED')) return { word: 'Published', tone: 'done' };
  if (has('PUBLISHED')) return { word: 'Partly published', tone: 'done' };
  return { word: 'Cancelled', tone: 'stopped' };
}

/**
 * The release's state at a glance (BLD-080): building while any job is under way, then queued, then failed when any
 * job failed, published when all did, and cancelled when none published.
 */
export function releaseSummary(jobs: readonly Pick<JobView, 'state'>[]): ReleaseSummary {
  const done = jobs.filter((job) => job.state === 'PUBLISHED').length;
  return { ...summaryWord(jobs), published: `${String(done)} of ${String(jobs.length)} published` };
}
