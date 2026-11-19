import type { Container, ContainerCalls, ContainerLog } from './api';
import { DEFAULT_LINES, type LineChoice } from './rows';

export type ReadStatus = 'idle' | 'loading' | 'ready' | 'failed';

/**
 * The containers of one device and the log of the one the operator selected (HOST-013). The hub reads the device at
 * each call, so nothing here reads again by itself: the operator asks.
 */
export class ContainersStore {
  status = $state<ReadStatus>('idle');
  /** Why the list did not load, in the hub's words: it says what to do (no SSH credential, no Docker). */
  problem = $state('');
  containers = $state.raw<readonly Container[]>([]);

  /** The identifier of the container whose log is shown; null when none is. */
  selected = $state<string | null>(null);
  lines = $state<LineChoice>(DEFAULT_LINES);
  logStatus = $state<ReadStatus>('idle');
  logProblem = $state('');
  log = $state.raw<ContainerLog | null>(null);

  #listGeneration = 0;
  #logGeneration = 0;

  constructor(private readonly calls: ContainerCalls) {}

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
    this.log = null;
    await this.readLog();
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

  /** Closes the log. The lines are dropped: the client keeps no log of a container. */
  close(): void {
    this.#logGeneration += 1;
    this.selected = null;
    this.log = null;
    this.logStatus = 'idle';
    this.logProblem = '';
  }
}
