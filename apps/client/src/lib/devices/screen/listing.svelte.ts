import { untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { addBeat, beatOf, latestBeats, type Beat } from '$lib/checks/heartbeats';
import type { Hub, HubError } from '$lib/hub/hub';
import type { ReadingsChange, StatusChange } from '../api';
import { devicesApi } from '../client';
import { appendPage, applyReadings, applyStatus, applyVncEnabled, fromPage, type DeviceList } from '../list';
import { DEFAULT_QUERY, listVariables, type ListQuery } from '../query';
import { coalesced } from '../live/coalesce';
import { watchLive } from '../live/live';
import { LiveCharges } from '$lib/metrics/battery/charges.svelte';
import { LiveMemory } from '$lib/metrics/container/memory.svelte';
import { isHubDevice } from '../actions/actions';
import { browserClock, skeletonGate, type LoadingClock, type SkeletonGate } from '$lib/motion/loading/loading';

/**
 * The devices the page shows and the hub calls behind them: the loaded list, the beats of each device,
 * the skeleton flag and the errors. `load` follows the URL query; `start` and `stop` bracket the live updates.
 */
export class Listing {
  list = $state.raw<DeviceList | null>(null);
  error = $state.raw<HubError | null>(null);
  moreError = $state.raw<HubError | null>(null);
  liveError = $state.raw<HubError | null>(null);
  loadingMore = $state(false);
  skeleton = $state(false);
  readonly beats = new SvelteMap<string, readonly Beat[]>();
  /** The latest charge of each battery of each device, from the batches since `start` (MET-032). */
  readonly charges = new LiveCharges();
  /** The bytes of memory in use of the hub's own device, for its memory as used of its limit (HMD-073). */
  readonly memory = new LiveMemory();

  readonly #hub: Hub;
  readonly #devices;
  readonly #gate: SkeletonGate;
  readonly #refresh = coalesced(
    () => this.#refreshOnce(),
    () => this.#stopped,
  );
  #stopLive: (() => void) | undefined;
  #query: ListQuery = DEFAULT_QUERY;
  #loadedKey: string | undefined;
  /** Counts the loads that replace the list; an answer from an earlier one is dropped. */
  #generation = 0;
  #stopped = false;

  constructor(hub: Hub, clock: LoadingClock = browserClock) {
    this.#hub = hub;
    this.#devices = devicesApi(hub);
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
    if (mine !== this.#generation || this.list === null) {
      this.loadingMore = false;
      return;
    }
    this.loadingMore = false;
    if (result.ok) {
      this.moreError = null;
      this.#show(appendPage(this.list, result.value));
    } else {
      this.moreError = result.error;
    }
  }

  /** Shows device `id` with VNC on, without loading the list again (R-079). */
  enableVnc(id: string): void {
    if (this.list !== null) this.list = applyVncEnabled(this.list, id);
  }

  /** Starts the live status, readings and heartbeat updates; `stop` ends them. */
  start(): void {
    this.#stopped = false;
    this.charges.start(this.#hub);
    this.#watchHubDevice();
    this.#stopLive = watchLive(this.#hub, {
      onStatus: (change) => {
        this.#onStatus(change);
      },
      onReadings: (change) => {
        this.#onReadings(change);
      },
      onBeat: (added) => {
        const known = this.beats.get(added.deviceId);
        if (known === undefined) return;
        const next = addBeat(known, beatOf(added));
        if (next !== known) this.beats.set(added.deviceId, next);
      },
      onError: (failure) => {
        this.liveError = failure;
      },
    });
  }

  /** Ends the live updates and drops every answer still on its way; a later `load` fetches again. */
  stop(): void {
    this.#stopped = true;
    this.#stopLive?.();
    this.#stopLive = undefined;
    this.charges.stop();
    this.memory.stop();
    this.#gate.stop();
    this.#generation += 1;
    this.#loadedKey = undefined;
    this.loadingMore = false;
  }

  #show(next: DeviceList): void {
    this.list = next;
    for (const device of next.devices) {
      if (!this.beats.has(device.id)) this.beats.set(device.id, latestBeats(device.checks));
    }
    this.#watchHubDevice();
  }

  /**
   * Listens to the batches of the hub's own device while the list holds it and the live updates run. `start` runs in
   * an effect: the list is read untracked, so a new list does not restart the live updates.
   */
  #watchHubDevice(): void {
    const hubDevice = untrack(() => this.list)?.devices.find(isHubDevice);
    if (hubDevice !== undefined && !this.#stopped) this.memory.watch(this.#hub, hubDevice.id);
  }

  async #loadFirst(): Promise<void> {
    this.#generation += 1;
    const mine = this.#generation;
    this.loadingMore = false;
    // The skeleton stands in for a list that is not there; a list on screen stays while the next one loads.
    if (this.list === null) this.#gate.begin();
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

  /** Fetches the first page again without the skeleton: a device entered the filter (DEV-003). It drops a Load more in flight. */
  async #refreshOnce(): Promise<void> {
    this.#generation += 1;
    const mine = this.#generation;
    this.loadingMore = false;
    const result = await this.#devices.list(listVariables(this.#query));
    if (mine !== this.#generation) return;
    this.#gate.end();
    if (result.ok) {
      this.beats.clear();
      this.#show(fromPage(result.value));
    }
  }

  /**
   * Shows the new readings. A device that the list holds with no metrics source has gained one since the list loaded,
   * for example a daemon that enrolled meanwhile: the first page loads again, so that its row shows its bars (MET-007).
   */
  #onReadings(change: ReadingsChange): void {
    if (this.list === null) return;
    const listed = this.list.devices.find((device) => device.id === change.deviceId);
    this.list = applyReadings(this.list, change);
    if (listed?.metricsSource === 'NONE') void this.#refresh();
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
