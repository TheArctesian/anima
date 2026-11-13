import type { Builder } from '../api';
import type { BuildTarget, JobChange, JobView, ReleaseView } from './api';

/** The targets a release can be built for, with the names the operator reads (BLD-001). */
export const TARGETS: readonly { readonly value: BuildTarget; readonly label: string }[] = [
  { value: 'LINUX_X86_64', label: 'linux-x86_64' },
  { value: 'DARWIN_UNIVERSAL', label: 'darwin-universal' },
];

const STATE_LABELS: Record<JobView['state'], string> = {
  QUEUED: 'Queued',
  CLAIMED: 'Claimed',
  BUILDING: 'Building',
  UPLOADING: 'Uploading',
  PUBLISHED: 'Published',
  FAILED: 'Failed',
  CANCELLED: 'Cancelled',
};

const SHORT_COMMIT_LENGTH = 7;
const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

export function targetLabel(target: BuildTarget): string {
  return TARGETS.find((known) => known.value === target)?.label ?? target;
}

export function stateLabel(state: JobView['state']): string {
  return STATE_LABELS[state];
}

/** The first 7 characters of a commit, as Git shows it. */
export function shortCommit(commit: string): string {
  return commit.slice(0, SHORT_COMMIT_LENGTH);
}

/** Releases newest first; releases made at the same moment keep the hub's order (BLD-080). */
export function newestFirst(releases: readonly ReleaseView[]): ReleaseView[] {
  return releases.toSorted((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt));
}

function counted(count: number, unit: string): string {
  return count === 1 ? `1 ${unit}` : `${String(count)} ${unit}s`;
}

/** How long the job has been in its state, in whole units rounded down: "under a minute", "3 minutes", "2 hours" (BLD-080). */
export function timeInState(job: Pick<JobView, 'stateChangedAt'>, now: Date): string {
  const elapsed = now.getTime() - Date.parse(job.stateChangedAt);
  if (!(elapsed >= MINUTE_MS)) return 'under a minute';
  if (elapsed < HOUR_MS) return counted(Math.floor(elapsed / MINUTE_MS), 'minute');
  if (elapsed < DAY_MS) return counted(Math.floor(elapsed / HOUR_MS), 'hour');
  return counted(Math.floor(elapsed / DAY_MS), 'day');
}

/** Whether a builder with this platform can build the target (BLD-003). */
function builds(builder: Builder, target: BuildTarget): boolean {
  const { os, arch } = builder.platform;
  return target === 'LINUX_X86_64' ? os === 'LINUX' && arch === 'X86_64' : os === 'MACOS' && arch === 'AARCH64';
}

/**
 * "Waiting for a <target> builder" for a queued job when no online builder can build its target; otherwise `null`.
 * `builders` is `null` until they have loaded once, and then nothing is said (BLD-007).
 */
export function waitingText(job: Pick<JobView, 'state' | 'target'>, builders: readonly Builder[] | null): string | null {
  if (job.state !== 'QUEUED' || builders === null) return null;
  if (builders.some((builder) => builder.online && builds(builder, job.target))) return null;
  return `Waiting for a ${targetLabel(job.target)} builder`;
}

/** Only a queued job can be cancelled (BLD-008). */
export function canCancel(job: Pick<JobView, 'state'>): boolean {
  return job.state === 'QUEUED';
}

/** `releases` with the changed job replaced where it is listed; a job the list does not hold changes nothing. */
export function withChange(releases: readonly ReleaseView[], change: JobChange): readonly ReleaseView[] {
  return releases.map((release) =>
    release.id === change.job.releaseId
      ? { ...release, jobs: release.jobs.map((job) => (job.id === change.job.id ? change.job : job)) }
      : release,
  );
}

/** The releases of one version: the newest, and the earlier ones it replaced (BLD-080). */
export interface VersionAttempts {
  readonly latest: ReleaseView;
  /** Newest first. */
  readonly earlier: readonly ReleaseView[];
}

/** `releases` (newest first) grouped by version, each group in the place of its newest release. */
export function attemptsByVersion(releases: readonly ReleaseView[]): VersionAttempts[] {
  const groups = new Map<string, { latest: ReleaseView; earlier: ReleaseView[] }>();
  for (const release of releases) {
    const group = groups.get(release.version);
    if (group === undefined) groups.set(release.version, { latest: release, earlier: [] });
    else group.earlier.push(release);
  }
  return [...groups.values()];
}

/** Whether a job has stopped changing: published, failed or cancelled. */
export function isEnded(job: Pick<JobView, 'state'>): boolean {
  return job.state === 'PUBLISHED' || job.state === 'FAILED' || job.state === 'CANCELLED';
}

/** How long ago a job reached its state, worded for the state: "for 6 minutes" while it runs, "18 minutes ago" once it ended. */
export function stateAge(job: Pick<JobView, 'state' | 'stateChangedAt'>, now: Date): string {
  const span = timeInState(job, now);
  if (!isEnded(job)) return `for ${span}`;
  return span === 'under a minute' ? 'just now' : `${span} ago`;
}
