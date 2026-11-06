import type { HubError, HubResult } from '$lib/hub/hub';

/** A K1 connection page, newest first, as the heartbeat and session queries answer it. */
export interface Connection<T> {
  readonly pageInfo: { readonly hasNextPage: boolean; readonly endCursor?: string | null };
  readonly edges: readonly { readonly node: T }[];
}

/** Loads one page after `after` (null for the first); a `null` value means the owner is gone, so the list is empty. */
export type PageLoader<T> = (after: string | null) => Promise<HubResult<Connection<T> | null>>;

/**
 * A history read in pages, newest first, with "Load older" (D-10): the heartbeats of a check, the sessions of a
 * device. Live items join at the top through `prepend`. An answer that arrives after `stop` is dropped.
 */
export class Pages<T> {
  items = $state.raw<readonly T[] | null>(null);
  error = $state.raw<HubError | null>(null);
  olderError = $state.raw<HubError | null>(null);
  hasOlder = $state(false);
  loadingOlder = $state(false);

  readonly #load: PageLoader<T>;
  #cursor: string | null = null;
  #generation = 0;

  constructor(load: PageLoader<T>) {
    this.#load = load;
  }

  /** Loads the first page; again after a failure. */
  async load(): Promise<void> {
    this.#generation += 1;
    const mine = this.#generation;
    const result = await this.#load(null);
    if (mine !== this.#generation) return;
    if (!result.ok) {
      this.error = result.error;
      return;
    }
    this.error = null;
    this.items = [];
    this.#append(result.value);
  }

  async loadOlder(): Promise<void> {
    if (this.items === null || !this.hasOlder || this.loadingOlder) return;
    this.loadingOlder = true;
    const mine = this.#generation;
    const result = await this.#load(this.#cursor);
    if (mine !== this.#generation) return;
    this.loadingOlder = false;
    this.olderError = result.ok ? null : result.error;
    if (result.ok) this.#append(result.value);
  }

  /** Puts a live item on top unless `same` finds it there already; nothing happens before the first page. */
  prepend(item: T, same: (a: T, b: T) => boolean): void {
    if (this.items === null || this.items.some((known) => same(known, item))) return;
    this.items = [item, ...this.items];
  }

  stop(): void {
    this.#generation += 1;
    this.loadingOlder = false;
  }

  #append(page: Connection<T> | null): void {
    this.items = [...(this.items ?? []), ...(page?.edges.map((edge) => edge.node) ?? [])];
    this.hasOlder = page?.pageInfo.hasNextPage ?? false;
    this.#cursor = page?.pageInfo.endCursor ?? null;
  }
}
