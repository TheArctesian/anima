import { checksApi, type DeviceCheck, type HeartbeatPage, type NewHeartbeat } from '$lib/checks/api';
import type { DeviceDetail, StatusChange } from '$lib/devices/api';
import { devicesApi } from '$lib/devices/client';
import { watchLive } from '$lib/devices/live/live';
import { DEFAULT_LOG_FILTER } from '$lib/events/log';
import { LogFeed } from '$lib/events/panel/feed.svelte';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { MetricRange } from '$lib/metrics/api';
import { sessionsApi } from '$lib/sessions/api';
import type { DeviceId } from './device-id';
import { DeviceMetrics } from './overview/metrics.svelte';
import { Pages } from './pages.svelte';
import type { DeviceSession } from './sessions/sessions';
import type { TabName } from './tabs/tabs';

/** One heartbeat in a check's history (CHK-003). */
export type HistoryBeat = NonNullable<HeartbeatPage>['edges'][number]['node'];

/** The heartbeat history of the check whose row is open (D-10). */
export interface OpenHistory {
  readonly checkId: string;
  readonly pages: Pages<HistoryBeat>;
}

/** Heartbeats per history page (D-10). */
export const HISTORY_PAGE = 100;

/** Sessions per page of the Sessions tab. */
const SESSION_PAGE = 50;

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
 * history, the live updates, and the data of each tab (D-5, D-9, D-10, D-11). The route builds it only for a
 * well-formed ID; a malformed one is "Device not found" without a `Detail` (D-14).
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

  readonly id: DeviceId;
  readonly metrics: DeviceMetrics;
  readonly sessionPages: Pages<DeviceSession>;
  readonly feed: LogFeed;
  readonly #hub: Hub;
  readonly #devices;
  readonly #checks;
  readonly #started: TabName[] = [];
  #stopLive: (() => void) | undefined;
  #checksAsked = false;
  #metricsKey = '';
  #stopped = false;

  constructor(hub: Hub, id: DeviceId, now: () => Date) {
    this.#hub = hub;
    this.#devices = devicesApi(hub);
    this.#checks = checksApi(hub);
    this.id = id;
    const connections = sessionsApi(hub);
    this.metrics = new DeviceMetrics(hub, id, now);
    this.sessionPages = new Pages<DeviceSession>((after) => connections.history(id, SESSION_PAGE, after));
    this.feed = new LogFeed(hub, now);
    this.feed.filter = { ...DEFAULT_LOG_FILTER, since: 'all', deviceId: id };
  }

  /** Loads the device and starts the live updates; `stop` ends both. */
  start(): void {
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
    this.metrics.stop();
    this.sessionPages.stop();
    this.feed.stop();
  }

  /** D-9: a panel's data loads the first time its tab shows. */
  showTab(tab: TabName): void {
    if (this.#started.includes(tab)) return;
    this.#started.push(tab);
    if (tab === 'overview' || tab === 'checks') void this.loadChecks();
    else if (tab === 'sessions') void this.sessionPages.load();
    else if (tab === 'logs') this.feed.start();
  }

  /** D-5: the Overview's metrics load when it shows and when the range or source changes; leaving it stops the live points. */
  showMetrics(tab: TabName, range: MetricRange): void {
    const source = this.device?.metricsSource ?? null;
    const key = `${range}:${source ?? ''}`;
    if (tab !== 'overview') {
      if (this.#metricsKey !== '') this.metrics.stop();
      this.#metricsKey = '';
      return;
    }
    if (source === null || key === this.#metricsKey) return;
    this.#metricsKey = key;
    void this.metrics.load(range, source);
  }

  async load(): Promise<void> {
    const result = await this.#devices.get(this.id);
    if (this.#stopped) return;
    this.error = result.ok ? null : result.error;
    if (!result.ok) return;
    this.device = result.value;
    this.missing = result.value === null;
  }

  /** Loads the checks the first time a tab needs them, and again after a failure. */
  async loadChecks(): Promise<void> {
    if ((this.#checksAsked && this.checksError === null)) return;
    this.#checksAsked = true;
    const result = await this.#checks.forDevice(this.id);
    if (this.#stopped) return;
    this.checksError = result.ok ? null : result.error;
    if (result.ok) this.checks = result.value ?? [];
  }

  setDevice(next: DeviceDetail): void {
    this.device = next;
  }

  /** DEV-014: confirmed in the Settings tab; the hub deletes the rest (DEV-015). `onDeleted` runs after the hub has. */
  async deleteDevice(onDeleted: () => void): Promise<HubResult<unknown>> {
    const result = await this.#devices.delete(this.id);
    if (result.ok) onDeleted();
    return result;
  }

  /** Puts the check a create, an update or a pause returns into the list, when that call succeeded. */
  async applyCheck(pending: Promise<HubResult<DeviceCheck>>): Promise<HubResult<unknown>> {
    const result = await pending;
    if (result.ok) this.putCheck(result.value);
    return result;
  }

  async deleteCheck(check: DeviceCheck): Promise<HubResult<unknown>> {
    const result = await this.#checks.delete(check.id);
    if (result.ok) this.dropCheck(check.id);
    return result;
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
    // A beat older than the top of the history is left out: the list stays newest first.
    const history = this.history?.checkId === added.checkId ? this.history.pages : null;
    if (history !== null && newer(beat, history.items?.[0])) history.prepend(beat, sameBeat);
  }
}
