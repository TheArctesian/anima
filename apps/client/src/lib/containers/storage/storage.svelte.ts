import type { HubResult } from '$lib/hub/hub';
import type { ContainerCalls, ContainerNetwork, ContainerStorage } from '../api';
import type { ReadStatus } from '../store.svelte';

/** One read of the hub that the operator starts: its state, its answer, and why it failed in the hub's words. */
class Read<T> {
  status = $state<ReadStatus>('idle');
  problem = $state('');
  value = $state.raw<T | null>(null);
  #generation = 0;

  constructor(private readonly ask: () => Promise<HubResult<T>>) {}

  /** Reads again; an answer to an earlier read is dropped. The last answer stays on screen while the next one loads. */
  async load(): Promise<void> {
    const generation = ++this.#generation;
    this.status = 'loading';
    const result = await this.ask();
    if (generation !== this.#generation) return;
    if (!result.ok) {
      this.problem = result.error.message;
      this.status = 'failed';
      return;
    }
    this.value = result.value;
    this.problem = '';
    this.status = 'ready';
  }

  /** Reads when nothing was read yet. */
  async open(): Promise<void> {
    if (this.status === 'idle') await this.load();
  }
}

/**
 * HOST-020: the images, the volumes and the networks of one device. The hub reads the device at each call, and the
 * read of the sizes is slow, so each list is read when the operator first opens it and again only when the operator asks.
 */
export class StorageStore {
  readonly storage: Read<ContainerStorage>;
  readonly networks: Read<readonly ContainerNetwork[]>;

  constructor(calls: Pick<ContainerCalls, 'storage' | 'networks'>) {
    this.storage = new Read(() => calls.storage());
    this.networks = new Read(() => calls.networks());
  }
}
