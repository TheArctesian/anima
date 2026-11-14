import type { Hub, HubError } from '$lib/hub/hub';
import { eventsApi, type LogEvent } from '../api';
import {
  DEFAULT_LOG_FILTER,
  addLive,
  appendOlder,
  eventVariables,
  fromPage,
  minSeverity,
  type EventLog,
  type LogFilter,
} from '../log';

/**
 * What the logs panel shows and the hub calls behind it (LOG-009, LOG-010): the filter, the rows, the errors.
 * A filter change loads the first page again and listens again with the new `minSeverity`.
 * `now` is read once for each load and each live event, so the time window follows the clock.
 */
export class LogFeed {
  filter = $state.raw<LogFilter>(DEFAULT_LOG_FILTER);
  log = $state.raw<EventLog | null>(null);
  error = $state.raw<HubError | null>(null);
  olderError = $state.raw<HubError | null>(null);
  liveError = $state.raw<HubError | null>(null);
  loadingOlder = $state(false);
  /** The id of the row that arrived live last; the panel moves and highlights it. */
  fresh = $state.raw<string | null>(null);
  /** Whether live rows join the list as they come; off while the operator is away from the newest rows or paused (LOG-011). */
  following = $state(true);
  /** Live rows that came while not following, newest first; `follow(true)` puts them in the list. */
  waiting = $state.raw<readonly LogEvent[]>([]);

  readonly #api;
  readonly #now: () => Date;
  #stopListening: (() => void) | undefined;
  /** Counts the loads; an answer from an earlier one is dropped. */
  #generation = 0;
  /** Live events that came while the first page was loading; they join the page when it arrives (LOG-010). */
  #held: LogEvent[] = [];
  #loading = false;

  constructor(hub: Hub, now: () => Date) {
    this.#api = eventsApi(hub);
    this.#now = now;
  }

  /** Loads the first page and starts listening; `stop` ends it. */
  start(): void {
    void this.#reload();
    this.#listen();
  }

  stop(): void {
    this.#stopListening?.();
    this.#stopListening = undefined;
    this.#generation += 1;
  }

  /** Rows of the old filter do not stay under the new controls: they go until the new page arrives (LOG-009). */
  setFilter(next: LogFilter): void {
    this.filter = next;
    this.log = null;
    this.fresh = null;
    void this.#reload();
    this.#listen();
  }

  /** Starts or stops following; starting puts the waiting rows in the list, and the newest of them is the fresh one (LOG-011). */
  follow(on: boolean): void {
    this.following = on;
    if (!on || this.waiting.length === 0 || this.log === null) return;
    const waiting = this.waiting;
    this.waiting = [];
    this.log = waiting.reduceRight((log, event) => addLive(log, event, this.filter, this.#now()), this.log);
    this.fresh = waiting[0]?.id ?? null;
  }

  /** Loads the first page again, after a failure. */
  retry(): Promise<void> {
    return this.#reload();
  }

  async loadOlder(): Promise<void> {
    const current = this.log;
    if (current === null || !current.hasNextPage || this.loadingOlder) return;
    this.loadingOlder = true;
    const mine = this.#generation;
    const result = await this.#api.list(eventVariables(this.filter, this.#now(), current.endCursor));
    this.loadingOlder = false;
    if (mine !== this.#generation || this.log === null) return;
    if (result.ok) {
      this.olderError = null;
      this.log = appendOlder(this.log, result.value);
    } else {
      this.olderError = result.error;
    }
  }

  async #reload(): Promise<void> {
    this.#generation += 1;
    const mine = this.#generation;
    this.#held = [];
    this.waiting = [];
    this.#loading = true;
    const result = await this.#api.list(eventVariables(this.filter, this.#now()));
    if (mine !== this.#generation) return;
    this.#loading = false;
    if (result.ok) {
      this.error = null;
      this.olderError = null;
      this.fresh = null;
      this.log = this.#held.reduce((log, held) => addLive(log, held, this.filter, this.#now()), fromPage(result.value));
    } else {
      this.error = result.error;
    }
  }

  #listen(): void {
    this.#stopListening?.();
    this.#stopListening = this.#api.watch(minSeverity(this.filter), {
      next: (event) => {
        this.#add(event);
      },
      error: (failure) => {
        this.liveError = failure;
      },
      complete: () => undefined,
    });
  }

  #add(event: LogEvent): void {
    if (this.#loading) this.#held.push(event);
    if (this.log === null) return;
    const next = addLive(this.log, event, this.filter, this.#now());
    if (next === this.log) return;
    if (!this.following) {
      if (!this.waiting.some((row) => row.id === event.id)) this.waiting = [event, ...this.waiting];
      return;
    }
    this.log = next;
    this.fresh = event.id;
  }
}
