import { SvelteMap } from 'svelte/reactivity';
import { checksApi } from '$lib/checks/api';
import { addBeat, beatOf, latestBeats, type Beat } from '$lib/checks/heartbeats';
import type { Hub, HubError } from '$lib/hub/hub';
import { devicesApi, type StatusChange } from '../api';
import { appendPage, applyStatus, fromPage, type DeviceList } from '../list';
import { DEFAULT_QUERY, listVariables, type ListQuery } from '../query';
import { browserClock, skeletonGate, type LoadingClock, type SkeletonGate } from './loading';

/**
 * What the Devices page shows, and the hub calls behind it: the loaded list, the beats of each device, the tags,
 * the skeleton flag and the errors. `load` follows the URL query; `start` and `stop` bracket the live updates.
 */
export class Listing {
  list = $state.raw<DeviceList | null>(null);
  error = $state.raw<HubError | null>(null);
  moreError = $state.raw<HubError | null>(null);
  liveError = $state.raw<HubError | null>(null);
  tags = $state.raw<readonly string[]>([]);
  tagsError = $state.raw<HubError | null>(null);
  loadingMore = $state(false);
  skeleton = $state(false);
  readonly beats = new SvelteMap<string, readonly Beat[]>();

  readonly #devices;
  readonly #checks;
  readonly #gate: SkeletonGate;
  readonly #stops: (() => void)[] = [];
  #query: ListQuery = DEFAULT_QUERY;
  #loadedKey: string | undefined;
  /** Counts the loads that follow the query; an answer from an earlier one is dropped. */
  #generation = 0;
  #refreshing = false;
  #refreshAgain = false;

  constructor(hub: Hub, clock: LoadingClock = browserClock) {
    this.#devices = devicesApi(hub);
    this.#checks = checksApi(hub);
    this.#gate = skeletonGate(clock, (visible) => {
      this.skeleton = visible;
    });
  }

  /** Shows the devices for `next`; the same hub variables as before do not load again, so a view change is free. */
  load(next: ListQuery): Promise<void> {
    const key = JSON.stringify(listVariables(next));
    this.#query = next;
    if (key === this.#loadedKey) return Promise.resolve();
    this.#loadedKey = key;
    return this.#loadFirst();
  }

  /** Loads the first page again after a failure. */
  retry(): Promise<void> {
    return this.#loadFirst();
  }

  async loadMore(): Promise<void> {
    const current = this.list;
    if (current === null || !current.hasNextPage || this.loadingMore) return;
    this.loadingMore = true;
    const mine = this.#generation;
    const result = await this.#devices.list(listVariables(this.#query, current.endCursor));
    this.loadingMore = false;
    if (mine !== this.#generation || this.list === null) return;
    if (result.ok) {
      this.moreError = null;
      this.#show(appendPage(this.list, result.value));
    } else {
      this.moreError = result.error;
    }
  }

  /** Starts the live status and heartbeat updates and loads the tags; `stop` ends them. */
  start(): void {
    this.#stops.push(
      this.#devices.watchStatus({
        next: (change) => {
          this.#onStatus(change);
        },
        error: (failure) => {
          this.liveError = failure;
        },
        complete: () => undefined,
      }),
      this.#checks.watchHeartbeats({
        next: (added) => {
          const known = this.beats.get(added.deviceId);
          if (known === undefined) return;
          const next = addBeat(known, beatOf(added));
          if (next !== known) this.beats.set(added.deviceId, next);
        },
        error: (failure) => {
          this.liveError = failure;
        },
        complete: () => undefined,
      }),
    );
    void this.#devices.tags().then((result) => {
      if (result.ok) this.tags = result.value;
      else this.tagsError = result.error;
    });
  }

  stop(): void {
    for (const stopListening of this.#stops.splice(0)) stopListening();
    this.#gate.stop();
    this.#generation += 1;
  }

  #show(next: DeviceList): void {
    this.list = next;
    for (const device of next.devices) {
      if (!this.beats.has(device.id)) this.beats.set(device.id, latestBeats(device.checks));
    }
  }

  async #loadFirst(): Promise<void> {
    this.#generation += 1;
    const mine = this.#generation;
    this.#gate.begin();
    const result = await this.#devices.list(listVariables(this.#query));
    if (mine !== this.#generation) return;
    this.#gate.end();
    if (result.ok) {
      this.error = null;
      this.moreError = null;
      this.beats.clear();
      this.#show(fromPage(result.value));
    } else {
      this.error = result.error;
    }
  }

  /** Fetches the first page again without the skeleton: a device entered the filter (DEV-003). */
  async #refresh(): Promise<void> {
    if (this.#refreshing) {
      this.#refreshAgain = true;
      return;
    }
    this.#refreshing = true;
    const mine = this.#generation;
    const result = await this.#devices.list(listVariables(this.#query));
    this.#refreshing = false;
    if (mine === this.#generation && result.ok) {
      this.beats.clear();
      this.#show(fromPage(result.value));
    }
    if (this.#refreshAgain) {
      this.#refreshAgain = false;
      await this.#refresh();
    }
  }

  #onStatus(change: StatusChange): void {
    if (this.list === null) return;
    if (this.list.devices.some((device) => device.id === change.deviceId)) {
      this.list = applyStatus(this.list, change, this.#query);
    } else if (this.#query.statuses.length > 0) {
      void this.#refresh();
    }
  }
}
