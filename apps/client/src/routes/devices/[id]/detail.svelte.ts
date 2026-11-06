import { checksApi, type DeviceCheck, type HeartbeatPage, type NewHeartbeat } from '$lib/checks/api';
import { devicesApi, type DeviceDetail, type StatusChange } from '$lib/devices/api';
import { watchLive } from '$lib/devices/live/live';
import type { Hub, HubError } from '$lib/hub/hub';
import type { DeviceId } from './device-id';
import { Pages } from './pages.svelte';

/** One heartbeat in a check's history (CHK-003). */
export type HistoryBeat = NonNullable<HeartbeatPage>['edges'][number]['node'];

/** The heartbeat history of the check whose row is open (D-10). */
export interface OpenHistory {
  readonly checkId: string;
  readonly pages: Pages<HistoryBeat>;
}

/** Heartbeats per history page (D-10). */
export const HISTORY_PAGE = 100;

/** Heartbeats have no ID; one check records at most one per instant. */
function sameBeat(a: HistoryBeat, b: HistoryBeat): boolean {
  return Date.parse(a.at) === Date.parse(b.at);
}

/** Whether `beat` is newer than the check's last one; a late beat does not overwrite the row. */
function newer(beat: HistoryBeat, last: HistoryBeat | null | undefined): boolean {
  return last == null || !(Date.parse(beat.at) < Date.parse(last.at));
}

/**
 * The device the detail page shows and the hub calls behind it: the device, its checks, the open heartbeat
 * history and the live updates (D-10, D-11). A null ID loads nothing: the page is "Device not found" (D-14).
 */
export class Detail {
  device = $state.raw<DeviceDetail | null>(null);
  /** The hub has no such device, or the ID is malformed. */
  missing = $state(false);
  error = $state.raw<HubError | null>(null);
  checks = $state.raw<readonly DeviceCheck[] | null>(null);
  checksError = $state.raw<HubError | null>(null);
  liveError = $state.raw<HubError | null>(null);
  history = $state.raw<OpenHistory | null>(null);

  readonly id: DeviceId | null;
  readonly #hub: Hub;
  readonly #devices;
  readonly #checks;
  #stopLive: (() => void) | undefined;
  #checksAsked = false;
  #stopped = false;

  constructor(hub: Hub, id: DeviceId | null) {
    this.#hub = hub;
    this.#devices = devicesApi(hub);
    this.#checks = checksApi(hub);
    this.id = id;
    this.missing = id === null;
  }

  /** Loads the device and starts the live updates; `stop` ends both. */
  start(): void {
    if (this.id === null) return;
    this.#stopped = false;
    void this.load();
    this.#stopLive = watchLive(this.#hub, {
      onStatus: (change) => {
        this.#onStatus(change);
      },
      onBeat: (added) => {
        this.#onBeat(added);
      },
      onError: (failure) => {
        this.liveError = failure;
      },
    });
  }

  /** Ends the live updates; answers still on their way are dropped (D-11). */
  stop(): void {
    this.#stopped = true;
    this.#stopLive?.();
    this.#stopLive = undefined;
    this.history?.pages.stop();
  }

  async load(): Promise<void> {
    if (this.id === null) return;
    const result = await this.#devices.get(this.id);
    if (this.#stopped) return;
    this.error = result.ok ? null : result.error;
    if (!result.ok) return;
    this.device = result.value;
    this.missing = result.value === null;
  }

  /** Loads the checks the first time a tab needs them, and again after a failure. */
  async loadChecks(): Promise<void> {
    if (this.id === null || (this.#checksAsked && this.checksError === null)) return;
    this.#checksAsked = true;
    const result = await this.#checks.forDevice(this.id);
    if (this.#stopped) return;
    this.checksError = result.ok ? null : result.error;
    if (result.ok) this.checks = result.value ?? [];
  }

  setDevice(next: DeviceDetail): void {
    this.device = next;
  }

  /** A check the hub returned after a create, an update or a pause: it replaces the old one or joins the list. */
  putCheck(check: DeviceCheck): void {
    const known = this.checks ?? [];
    this.checks = known.some((other) => other.id === check.id)
      ? known.map((other) => (other.id === check.id ? check : other))
      : [...known, check];
  }

  dropCheck(id: string): void {
    this.checks = (this.checks ?? []).filter((check) => check.id !== id);
    if (this.history?.checkId === id) this.closeHistory();
  }

  /** Opens one check's history and loads its first page; another open history closes (D-10). */
  openHistory(checkId: string): void {
    if (this.history?.checkId === checkId) return;
    this.history?.pages.stop();
    const pages = new Pages<HistoryBeat>((after) => this.#checks.heartbeats(checkId, HISTORY_PAGE, after));
    this.history = { checkId, pages };
    void pages.load();
  }

  closeHistory(): void {
    this.history?.pages.stop();
    this.history = null;
  }

  #onStatus(change: StatusChange): void {
    if (change.deviceId !== this.id || this.device === null) return;
    this.device = { ...this.device, status: change.status };
  }

  /** A new heartbeat updates its check's row and the open history (D-11, New heartbeat role). */
  #onBeat(added: NewHeartbeat): void {
    if (added.deviceId !== this.id) return;
    const beat: HistoryBeat = { ...added.heartbeat, message: null };
    if (this.checks !== null) {
      this.checks = this.checks.map((check) =>
        check.id === added.checkId && newer(beat, check.lastHeartbeat) ? { ...check, status: beat.status, lastHeartbeat: beat } : check,
      );
    }
    if (this.history?.checkId === added.checkId) this.history.pages.prepend(beat, sameBeat);
  }
}
