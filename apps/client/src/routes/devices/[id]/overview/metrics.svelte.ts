import type { TimeWindow } from '$lib/charts/scales';
import type { Hub, HubError } from '$lib/hub/hub';
import type { MetricsSource } from '$lib/hub/generated/graphql';
import { metricsApi, type DeviceMetricsView, type MetricRange, type NewMetrics } from '$lib/metrics/api';
import { mergeLive, rangeWindow } from '$lib/metrics/live';

/**
 * The metrics the Overview shows for one device and the hub calls behind them (D-5, D-6): the series of the chosen
 * range, the window they are drawn in, the latest readings and the live points of the 1-hour range. The window moves
 * only when data arrives (a load or a sample), never with the clock, so a chart slides once per new point (5.7.3).
 */
export class DeviceMetrics {
  view = $state.raw<DeviceMetricsView | null>(null);
  /** The window the chart draws, `null` before the first answer. */
  span = $state.raw<TimeWindow | null>(null);
  /** The newest readings a live batch carried; kept across range changes. */
  readings = $state.raw<NewMetrics['readings'] | null>(null);
  error = $state.raw<HubError | null>(null);
  liveError = $state.raw<HubError | null>(null);
  /** A request is on its way. */
  loading = $state(false);

  readonly #deviceId: string;
  readonly #api;
  readonly #now: () => Date;
  #unwatch: (() => void) | undefined;
  #shown: MetricRange | null = null;
  #generation = 0;
  #stopped = false;

  constructor(hub: Hub, deviceId: string, now: () => Date) {
    this.#deviceId = deviceId;
    this.#api = metricsApi(hub);
    this.#now = now;
  }

  /**
   * Shows `range`. Only an SSH device has metrics (D-7); any other source sends no request. The 1-hour range then
   * listens for live points. A newer call drops an older answer; so does `stop`.
   */
  async load(range: MetricRange, source: MetricsSource): Promise<void> {
    this.#generation += 1;
    const mine = this.#generation;
    this.#stopped = false;
    this.#unwatch?.();
    this.#unwatch = undefined;
    this.error = null;
    this.liveError = null;
    if (source !== 'SSH') {
      this.view = null;
      this.#shown = null;
      this.span = null;
      this.loading = false;
      return;
    }
    this.loading = true;
    if (range === 'HOUR') this.#unwatch = this.#watch();
    const at = this.#now();
    const result = await this.#api.series(this.#deviceId, range);
    if (mine !== this.#generation) return;
    this.loading = false;
    if (!result.ok) {
      this.error = result.error;
      return;
    }
    this.view = result.value;
    this.#shown = range;
    this.span = rangeWindow(range, at);
  }

  /** Ends the subscription; answers still on their way are dropped. */
  stop(): void {
    this.#generation += 1;
    this.#stopped = true;
    this.#unwatch?.();
    this.#unwatch = undefined;
    this.loading = false;
  }

  #watch(): () => void {
    return this.#api.watch(this.#deviceId, {
      next: (added) => {
        this.#add(added);
      },
      error: (failure) => {
        if (!this.#stopped) this.liveError = failure;
      },
      complete: () => undefined,
    });
  }

  /** The point joins the series and the window ends at it, so the chart slides by the distance it moved. */
  #add(added: NewMetrics): void {
    const at = Date.parse(added.at);
    if (this.#stopped || Number.isNaN(at)) return;
    this.readings = added.readings;
    // A DAY or WEEK view is an aggregate; a live point joins only the 1-hour one (D-5).
    if (this.#shown !== 'HOUR' || this.view === null || this.span === null || at <= this.span.end) return;
    this.view = { ...this.view, series: mergeLive(this.view.series, added) };
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a value for a pure function, never held or changed
    this.span = rangeWindow('HOUR', new Date(at));
  }
}
