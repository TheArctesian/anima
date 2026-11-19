import type { Container, ContainerCalls, ContainerDetail, ContainerLog, ImageUpdate } from './api';
import { DEFAULT_LINES, type LineChoice } from './rows';
import { StorageStore } from './storage/storage.svelte';

export type ReadStatus = 'idle' | 'loading' | 'ready' | 'failed';

/** What the section below the table shows of the selected container. */
export type Shown = 'log' | 'detail';

/**
 * The containers of one device and the log of the one the operator selected (HOST-013). The hub reads the device at
 * each call, so nothing here reads again by itself: the operator asks.
 */
export class ContainersStore {
  status = $state<ReadStatus>('idle');
  /** Why the list did not load, in the hub's words: it says what to do (no SSH credential, no Docker). */
  problem = $state('');
  containers = $state.raw<readonly Container[]>([]);

  /** The identifier of the container whose log or details are shown; null when none is. */
  selected = $state<string | null>(null);
  shown = $state<Shown>('log');
  lines = $state<LineChoice>(DEFAULT_LINES);
  logStatus = $state<ReadStatus>('idle');
  logProblem = $state('');
  log = $state.raw<ContainerLog | null>(null);

  /** HOST-017: the details of the selected container. */
  detailStatus = $state<ReadStatus>('idle');
  detailProblem = $state('');
  detail = $state.raw<ContainerDetail | null>(null);

  /** HOST-021: empty until the operator asks; the hub then asks public registries, which takes time. */
  updatesStatus = $state<ReadStatus>('idle');
  updatesProblem = $state('');
  updates = $state.raw<readonly ImageUpdate[]>([]);

  #listGeneration = 0;
  #detailGeneration = 0;
  #updatesGeneration = 0;
  #logGeneration = 0;

  /** HOST-020: the images, the volumes and the networks of the same device. */
  readonly storage: StorageStore;

  constructor(private readonly calls: ContainerCalls) {
    this.storage = new StorageStore(calls);
  }

  /** Reads the list; an answer to an earlier read is dropped. The last list stays on screen while the next one loads. */
  async load(): Promise<void> {
    const generation = ++this.#listGeneration;
    this.status = 'loading';
    const result = await this.calls.list();
    if (generation !== this.#listGeneration) return;
    if (!result.ok) {
      this.problem = result.error.message;
      this.status = 'failed';
      return;
    }
    this.containers = result.value;
    this.problem = '';
    this.status = 'ready';
    if (this.selected !== null && !result.value.some((container) => container.id === this.selected)) this.close();
  }

  /** Shows the log of a container and reads it. */
  async select(id: string): Promise<void> {
    this.selected = id;
    this.shown = 'log';
    this.log = null;
    await this.readLog();
  }

  /** Shows the details of a container and reads them. */
  async inspect(id: string): Promise<void> {
    this.selected = id;
    this.shown = 'detail';
    this.detail = null;
    await this.readDetail();
  }

  /** Reads the details of the selected container again. */
  async readDetail(): Promise<void> {
    const id = this.selected;
    if (id === null) return;
    const generation = ++this.#detailGeneration;
    this.detailStatus = 'loading';
    const result = await this.calls.detail(id);
    if (generation !== this.#detailGeneration) return;
    if (!result.ok) {
      this.detailProblem = result.error.message;
      this.detailStatus = 'failed';
      return;
    }
    this.detail = result.value;
    this.detailProblem = '';
    this.detailStatus = 'ready';
  }

  /** HOST-021: asks the hub whether a newer image exists for each container. Anima updates nothing. */
  async checkUpdates(): Promise<void> {
    const generation = ++this.#updatesGeneration;
    this.updatesStatus = 'loading';
    const result = await this.calls.updates();
    if (generation !== this.#updatesGeneration) return;
    if (!result.ok) {
      // An old answer beside the reason would read as the answer of this try.
      this.updates = [];
      this.updatesProblem = result.error.message;
      this.updatesStatus = 'failed';
      return;
    }
    this.updates = result.value;
    this.updatesProblem = '';
    this.updatesStatus = 'ready';
  }

  /** Changes how many lines the log shows and reads it again. */
  async setLines(lines: LineChoice): Promise<void> {
    this.lines = lines;
    if (this.selected !== null) await this.readLog();
  }

  /** Reads the log of the selected container again. */
  async readLog(): Promise<void> {
    const id = this.selected;
    if (id === null) return;
    const generation = ++this.#logGeneration;
    this.logStatus = 'loading';
    const result = await this.calls.log(id, Number(this.lines));
    if (generation !== this.#logGeneration) return;
    if (!result.ok) {
      this.logProblem = result.error.message;
      this.logStatus = 'failed';
      return;
    }
    this.log = result.value;
    this.logProblem = '';
    this.logStatus = 'ready';
  }

  /** Closes the log or the details. They are dropped: the client keeps no log and no details of a container. */
  close(): void {
    this.#logGeneration += 1;
    this.#detailGeneration += 1;
    this.detail = null;
    this.detailStatus = 'idle';
    this.detailProblem = '';
    this.selected = null;
    this.log = null;
    this.logStatus = 'idle';
    this.logProblem = '';
  }
}
