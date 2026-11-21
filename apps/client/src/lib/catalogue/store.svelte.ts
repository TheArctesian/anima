import type { CatalogueCalls, CatalogueEntry } from './api';
import { grouped, tierCounts, type Group, type TierChoice } from './entry';

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
  /** The search of the list: a part of a name or a tag. */
  search = $state('');
  /** The tier that the list shows (OD-HOST-50). */
  tier = $state<TierChoice>('ALL');
  #generation = 0;

  constructor(private readonly calls: CatalogueCalls) {}

  /** The entry that is shown, if it is still in the list. */
  get shown(): CatalogueEntry | null {
    return this.entries.find((entry) => entry.id === this.selected) ?? null;
  }

  /** The entries that the search and the tier keep, in groups by category (task H39). */
  get groups(): Group<CatalogueEntry>[] {
    return grouped(this.entries, this.search, this.tier);
  }

  /** How many entries each choice of the tier holds, before the search. */
  get counts(): Record<TierChoice, number> {
    return tierCounts(this.entries);
  }

  /** Shows the entries that hold `search` in their name or a tag, of the tier `tier`. */
  filter(search: string, tier: TierChoice): void {
    this.search = search;
    this.tier = tier;
  }

  /** Selects the entry `step` places after the selected one in the list as it is shown; the first one when none is. */
  move(step: number): void {
    const shown = this.groups.flatMap((group) => group.entries);
    if (shown.length === 0) return;
    const at = shown.findIndex((entry) => entry.id === this.selected);
    const next = at === -1 ? 0 : Math.min(Math.max(at + step, 0), shown.length - 1);
    this.selected = shown[next]?.id ?? null;
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
