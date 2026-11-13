import type { Hub, HubError } from '$lib/hub/hub';
import { buildsApi, type Builder } from '../api';
import { LOG_PAGE, releasesApi, type BuildTarget, type JobChange, type JobView, type LogLineView, type ReleaseView } from './api';
import { lastSequence, mergeLines } from './log';
import { newestFirst, withChange } from './release';

/** How often the view reads the builders and moves its clock: the hub marks a builder online for 60 seconds (BLD-007). */
export const REFRESH_MS = 30_000;

/**
 * What the Builds view shows and does: the releases, the builders, the selected job's log, and the calls that change
 * them (BLD-001, BLD-007, BLD-008, BLD-080, BLD-081). The hub's changes arrive through one subscription for every job.
 */
export class ReleaseBoard {
  releases = $state<readonly ReleaseView[] | null>(null);
  /** `null` until the builders have loaded once: without them nothing can be said about a waiting job (BLD-007). */
  builders = $state<readonly Builder[] | null>(null);
  buildersFailed = $state(false);
  /** The subscription ended, so the list and the log may be out of date until the operator resumes it (BLD-081). */
  liveStopped = $state(false);
  nextVersion = $state<string | null>(null);
  loadFailed = $state(false);
  /** The refusal of the last cancel, shown beside the job. */
  cancelError = $state<{ readonly jobId: string; readonly message: string } | null>(null);
  selectedId = $state<string | null>(null);
  logLines = $state<readonly LogLineView[]>([]);
  logFailed = $state(false);
  /** The time "time in state" is measured against, in ms. */
  nowMs = $state(0);

  readonly #api: ReturnType<typeof releasesApi>;
  readonly #builders: ReturnType<typeof buildsApi>;
  readonly #clock: () => number;
  #stop: (() => void) | null = null;

  constructor(hub: Hub, clock: () => number = Date.now) {
    this.#api = releasesApi(hub);
    this.#builders = buildsApi(hub);
    this.#clock = clock;
    this.nowMs = clock();
  }

  get now(): Date {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a fresh value read from state, never changed or held
    return new Date(this.nowMs);
  }

  /** The releases, newest first. */
  get ordered(): readonly ReleaseView[] {
    return newestFirst(this.releases ?? []);
  }

  get selected(): JobView | null {
    return this.releases?.flatMap((release) => release.jobs).find((job) => job.id === this.selectedId) ?? null;
  }

  /** Reads the releases, the builders and the proposed version, and starts the live changes; `stop` ends them. */
  async open(): Promise<void> {
    this.#watch();
    await Promise.all([this.loadReleases(), this.loadBuilders(), this.loadNextVersion()]);
  }

  /** Starts the live changes again after they stopped, and reads what was missed (BLD-081). */
  async resume(): Promise<void> {
    this.#watch();
    await this.#catchUp();
  }

  #watch(): void {
    if (this.#stop !== null) return;
    this.liveStopped = false;
    this.#stop = this.#api.watch(null, {
      next: (change) => {
        this.#apply(change);
      },
      error: () => {
        this.#ended();
      },
      complete: () => {
        this.#ended();
      },
    });
  }

  /** The hub closed or refused the subscription: say so, and read the releases and the log once, so nothing shown is stale. */
  #ended(): void {
    this.#stop = null;
    this.liveStopped = true;
    void this.#catchUp();
  }

  async #catchUp(): Promise<void> {
    const id = this.selectedId;
    await this.loadReleases();
    if (id !== null && this.selectedId === id) await this.#loadLog(id);
  }

  stop(): void {
    this.#stop?.();
    this.#stop = null;
  }

  async loadReleases(): Promise<void> {
    const result = await this.#api.list();
    this.loadFailed = !result.ok;
    if (result.ok) this.releases = result.value;
  }

  async loadBuilders(): Promise<void> {
    const result = await this.#builders.builders();
    this.buildersFailed = !result.ok;
    if (result.ok) this.builders = result.value;
  }

  async loadNextVersion(): Promise<void> {
    const result = await this.#api.nextVersion();
    if (result.ok) this.nextVersion = result.value;
  }

  /** The clock and the builders move on together, so "time in state" and "Waiting for a builder" stay true. */
  async tick(): Promise<void> {
    this.nowMs = this.#clock();
    await this.loadBuilders();
  }

  /** Starts a release; the hub's refusal comes back as an error for the form to show (BLD-001, BLD-002). */
  async release(version: string, commit: string, targets: readonly BuildTarget[]): Promise<HubError | null> {
    const result = await this.#api.create(version, commit, targets);
    if (!result.ok) return result.error;
    await Promise.all([this.loadReleases(), this.loadNextVersion()]);
    return null;
  }

  /** Cancels a queued job; the change arrives through the subscription, and a refusal is kept for the row (BLD-008). */
  async cancel(job: JobView): Promise<void> {
    const result = await this.#api.cancel(job.id);
    this.cancelError = result.ok ? null : { jobId: job.id, message: result.error.message };
    if (result.ok) await this.loadReleases();
  }

  /** Shows the log of `job`: the stored lines now, and the lines that arrive after (BLD-081). */
  async select(job: JobView): Promise<void> {
    this.selectedId = job.id;
    this.logLines = [];
    this.logFailed = false;
    await this.#loadLog(job.id);
  }

  async #loadLog(id: string): Promise<void> {
    for (;;) {
      const result = await this.#api.log(id, lastSequence(this.logLines));
      if (this.selectedId !== id) return;
      if (!result.ok) {
        this.logFailed = true;
        return;
      }
      this.logLines = mergeLines(this.logLines, result.value);
      if (result.value.length < LOG_PAGE) return;
    }
  }

  #apply(change: JobChange): void {
    if (this.releases !== null) this.releases = withChange(this.releases, change);
    if (change.job.id === this.selectedId) this.logLines = mergeLines(this.logLines, change.logLines);
    if (change.job.state === 'QUEUED') void this.loadBuilders();
    // A release made elsewhere: the list does not hold it yet.
    if (this.releases?.every((release) => release.id !== change.job.releaseId)) void this.loadReleases();
  }
}
