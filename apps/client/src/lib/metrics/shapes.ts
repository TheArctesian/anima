import type { Chart } from './overview-charts';

/** Series per chart of a device's last answer, by device id: what its loading skeleton must look like (MOT-007). */
const last = new Map<string, readonly number[]>();

/** Remember the shape of `charts` for `deviceId`; it outlives the page, so a revisit draws the right skeleton. */
export function rememberChartShape(deviceId: string, charts: readonly Pick<Chart, 'series'>[]): void {
  last.set(
    deviceId,
    charts.map((chart) => chart.series.length),
  );
}

/** The series count of each chart in the device's last answer, or `null` when there was none. */
export function chartShapeOf(deviceId: string): readonly number[] | null {
  return last.get(deviceId) ?? null;
}

/** Forget every device; for tests. */
export function forgetChartShapes(): void {
  last.clear();
}
