import type { CatalogueCalls, CatalogueEntry } from './api';

export type ReadStatus = 'idle' | 'loading' | 'ready' | 'failed';

/**
 * The catalogue of applications that ships with Anima (task H38). The hub reads it from its own binary, so it is read
 * when the operator first opens it and again only when the operator asks.
 */
export class CatalogueStore {
  status = $state<ReadStatus>('idle');
  /** Why the list did not load, in the hub's words. */
  problem = $state('');
  entries = $state.raw<readonly CatalogueEntry[]>([]);
  /** The identifier of the entry whose description is shown; null when none is. */
  selected = $state<string | null>(null);
  #generation = 0;

  constructor(private readonly calls: CatalogueCalls) {}

  /** The entry that is shown, if it is still in the list. */
  get shown(): CatalogueEntry | null {
    return this.entries.find((entry) => entry.id === this.selected) ?? null;
  }

  /** Reads the list; an answer to an earlier read is dropped. */
  async load(): Promise<void> {
    const generation = ++this.#generation;
    this.status = 'loading';
    const result = await this.calls.entries();
    if (generation !== this.#generation) return;
    if (!result.ok) {
      this.problem = result.error.message;
      this.status = 'failed';
      return;
    }
    this.entries = result.value;
    this.problem = '';
    this.status = 'ready';
  }

  /** Reads when nothing was read yet. */
  async open(): Promise<void> {
    if (this.status === 'idle') await this.load();
  }

  select(id: string | null): void {
    this.selected = id;
  }
}
