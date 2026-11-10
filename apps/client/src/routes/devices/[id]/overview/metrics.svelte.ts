import type { TimeWindow } from '$lib/charts/scales';
import type { Hub, HubError } from '$lib/hub/hub';
import type { MetricsSource } from '$lib/hub/generated/graphql';
import type { BarReadings } from '$lib/metrics/bars/bars';
import { metricsApi, type DeviceMetricsView, type MetricRange, type NewMetrics } from '$lib/metrics/api';
import { chartsOf } from '$lib/metrics/overview-charts';
import { chartShapeOf, rememberChartShape } from '$lib/metrics/shapes';
import { mergeLive } from '$lib/metrics/live';
import { rangeWindow } from '$lib/metrics/ranges';
import { hasMetrics } from '$lib/metrics/source/has-metrics';

/**
 * The metrics the Overview shows for one device and the hub calls behind them (D-5, D-6): the series of the chosen
 * range, the window they are drawn in, the latest readings and the live points of the 1-hour range. The window moves
 * only when data arrives (a load or a sample), never with the clock, so a chart slides once per new point (5.7.3).
 */
export class DeviceMetrics {
  view = $state.raw<DeviceMetricsView | null>(null);
  /** The window the chart draws, `null` before the first answer. */
  span = $state.raw<TimeWindow | null>(null);
  /** The range of the data on screen: it stays the old range while a switch is on its way. */
  get shownRange(): MetricRange | null {
    return this.#shown;
  }

  /** The series count of each chart the last answer for this device drew, `null` before one: the skeleton copies it. */
  chartShape = $state.raw<readonly number[] | null>(null);

  /** The hub's latest readings at load, then the newest a live batch carried; the bars show them. */
  readings = $state.raw<BarReadings | null>(null);
  error = $state.raw<HubError | null>(null);
  liveError = $state.raw<HubError | null>(null);

  readonly #deviceId: string;
  readonly #api;
  readonly #now: () => Date;
  #unwatch: (() => void) | undefined;
  #shown = $state.raw<MetricRange | null>(null);
  #generation = 0;
  #stopped = false;

  constructor(hub: Hub, deviceId: string, now: () => Date) {
    this.#deviceId = deviceId;
    this.#api = metricsApi(hub);
    this.#now = now;
    this.chartShape = chartShapeOf(deviceId);
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
    if (!hasMetrics(source)) {
      this.view = null;
      this.#shown = null;
      this.span = null;
      return;
    }
    if (range === 'HOUR') this.#unwatch = this.#watch();
    const at = this.#now();
    const result = await this.#api.series(this.#deviceId, range);
    if (mine !== this.#generation) return;
    if (!result.ok) {
      this.error = result.error;
      this.view = null;
      this.span = null;
      this.#shown = null;
      return;
    }
    this.view = result.value;
    if (result.value !== null) {
      const charts = chartsOf(result.value.series, result.value.gpus);
      rememberChartShape(this.#deviceId, charts);
      this.chartShape = chartShapeOf(this.#deviceId);
    }
    this.readings = result.value?.readings ?? this.readings;
    this.#shown = range;
    this.span = rangeWindow(range, at);
  }

  /** Ends the subscription; answers still on their way are dropped. */
  stop(): void {
    this.#generation += 1;
    this.#stopped = true;
    this.#unwatch?.();
    this.#unwatch = undefined;
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
    if (this.#shown !== 'HOUR' || this.view === null || this.span === null) return;
    // mergeLive drops a stale point per series, so the client clock decides nothing here: only the window end moves, and only forward.
    this.view = { ...this.view, series: mergeLive(this.view.series, added) };
    if (at <= this.span.end) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a value for a pure function, never held or changed
    this.span = rangeWindow('HOUR', new Date(at));
  }
}
