import { createContext } from 'svelte';
import type { Extension, ExtensionsApi } from './api';

export type ExtensionsStatus = 'loading' | 'ready' | 'failed';

/** The change the hub refused, with the hub's words. */
export interface Refusal {
  readonly key: string;
  readonly message: string;
}

/**
 * The extensions the hub lists and which are on (EXT-001). `shown` lays a change on its way over the hub's list so a
 * control moves at once; a refused change drops out, the control goes back, and `refusal` holds the hub's words.
 */
export class ExtensionsStore {
  status = $state<ExtensionsStatus>('loading');
  loadProblem = $state('');
  refusal = $state.raw<Refusal | null>(null);
  #all = $state.raw<readonly Extension[]>([]);
  #pending = $state.raw<Readonly<Record<string, boolean>>>({});

  readonly shown = $derived(this.#all.map((each) => ({ ...each, on: this.#pending[each.key] ?? each.on })));

  constructor(private readonly api: ExtensionsApi) {}

  /** The hub's list with no change laid over it; EXT-011 asks about what is on now. */
  get settled(): readonly Extension[] {
    return this.#all;
  }

  /** Whether `key` is on; false until the list is read (EXT-002). */
  isOn(key: string): boolean {
    return this.shown.some((each) => each.key === key && each.on);
  }

  async load(): Promise<void> {
    const result = await this.api.list();
    if (result.ok) {
      this.#all = result.value;
      this.loadProblem = '';
      this.status = 'ready';
    } else {
      this.loadProblem = result.error.message;
      this.status = 'failed';
    }
  }

  /** Reads the list unless it is already read. */
  async ensureLoaded(): Promise<void> {
    if (this.status !== 'ready') await this.load();
  }

  /** Sends the change; the control shows it while it travels and goes back if the hub refuses. */
  async set(key: string, on: boolean): Promise<void> {
    this.refusal = null;
    this.#pending = { ...this.#pending, [key]: on };
    const result = await this.api.set(key, on);
    if (result.ok) {
      this.#all = this.#all.map((each) => result.value.find((changed) => changed.key === each.key) ?? each);
    } else {
      this.refusal = { key, message: result.error.message };
    }
    this.#pending = Object.fromEntries(Object.entries(this.#pending).filter(([pending]) => pending !== key));
    // What the other rows need changes with this one (EXT-012).
    if (result.ok) await this.load();
  }
}

const [getExtensions, setExtensions] = createContext<ExtensionsStore>();

/** Make `store` available below; call once in the root layout. */
export function provideExtensions(store: ExtensionsStore): void {
  setExtensions(store);
}

export function useExtensions(): ExtensionsStore {
  return getExtensions();
}
