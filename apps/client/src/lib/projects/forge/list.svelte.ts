import { dataAge, type HubError, type HubResult } from '$lib/hub/hub';
import type { PageAsk } from './api';
import type { ForgePage, ForgeRead } from './types';

/** "Read 2 minutes ago", or "Stale, read 2 minutes ago" when the hub's last read failed (PRJ-032). */
export function readWords(read: ForgeRead, now: Date): string {
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- read once to word the age, never kept or changed
  const age = dataAge(new Date(read.at), now);
  return read.stale ? `Stale, read ${age}` : `Read ${age}`;
}

/**
 * One list the hub reads from GitHub (PRJ-031, PRJ-032): its first page, more pages on request, Refresh that skips the
 * hub's cache and keeps the rows on screen meanwhile, and a row put back in place after a change. Rows are keyed, since
 * GitHub's pages can repeat a row when the list changes between pages.
 */
export class ForgeList<T, P extends ForgePage<T> = ForgePage<T>> {
  status = $state<'loading' | 'ready' | 'failed'>('loading');
  rows = $state.raw<readonly T[]>([]);
  read = $state.raw<ForgeRead | null>(null);
  error = $state.raw<HubError | null>(null);
  /** The whole last first page, for what a list answer carries beside its rows (the merge methods of pull requests). */
  first = $state.raw<P | null>(null);
  next = $state<string | null>(null);
  refreshing = $state(false);
  loadingMore = $state(false);

  readonly #fetch: (ask: PageAsk) => Promise<HubResult<P>>;
  readonly #key: (row: T) => string;
  #generation = 0;

  constructor(fetch: (ask: PageAsk) => Promise<HubResult<P>>, key: (row: T) => string) {
    this.#fetch = fetch;
    this.#key = key;
  }

  /** The first page; `fresh` asks GitHub again rather than the hub's cache. */
  async load(fresh = false): Promise<void> {
    this.#generation += 1;
    const mine = this.#generation;
    if (this.status === 'ready') this.refreshing = true;
    else this.status = 'loading';
    const result = await this.#fetch({ fresh });
    if (mine !== this.#generation) return;
    this.refreshing = false;
    if (result.ok) {
      this.first = result.value;
      this.rows = result.value.rows;
      this.read = result.value.read;
      this.next = result.value.next;
      this.error = null;
      this.status = 'ready';
    } else if (this.status === 'ready') {
      this.error = result.error;
    } else {
      this.error = result.error;
      this.status = 'failed';
    }
  }

  async more(): Promise<void> {
    if (this.next === null || this.loadingMore) return;
    const mine = this.#generation;
    this.loadingMore = true;
    const result = await this.#fetch({ after: this.next });
    this.loadingMore = false;
    if (mine !== this.#generation) return;
    if (!result.ok) {
      this.error = result.error;
      return;
    }
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a lookup for this call only, never rendered
    const seen = new Set(this.rows.map(this.#key));
    this.rows = [...this.rows, ...result.value.rows.filter((row) => !seen.has(this.#key(row)))];
    this.next = result.value.next;
  }

  /** Puts a changed row back in place, or first when it is new; `keep` false takes it out (a closed issue in Open). */
  put(row: T, keep = true): void {
    const key = this.#key(row);
    const at = this.rows.findIndex((each) => this.#key(each) === key);
    if (!keep) this.rows = this.rows.filter((each) => this.#key(each) !== key);
    else if (at === -1) this.rows = [row, ...this.rows];
    else this.rows = this.rows.map((each, index) => (index === at ? row : each));
  }
}
