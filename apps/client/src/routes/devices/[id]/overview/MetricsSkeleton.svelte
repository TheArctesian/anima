<script lang="ts">
  import { untrack } from 'svelte';
  import ChartSkeleton from '$lib/charts/ChartSkeleton.svelte';
  import type { SeriesColour } from '$lib/charts/plot';
  import type { MetricRange } from '$lib/metrics/api';
  import { DEGRADED_ABOVE, formatPercent } from '$lib/metrics/units';
  import { rangeWindow } from '$lib/metrics/ranges';
  import './geometry.css';

  interface Props {
    range: MetricRange;
    now: () => Date;
    /** The series count of each chart in the device's last answer; `null` before any, which draws the SSH basics. */
    cells?: readonly number[] | null;
  }

  const { range, now, cells = null }: Props = $props();
  // The window is fixed when the range is: a skeleton is not redrawn with the clock.
  const span = $derived(rangeWindow(range, untrack(now)));
  /** CPU, memory, disk and network: the charts every SSH device has. */
  const BASIC_CELLS: readonly number[] = [1, 1, 1, 1];
  const SHAPE = $derived(cells !== null && cells.length > 0 ? cells : BASIC_CELLS);
  /** A chart with several series has a legend row; the placeholder reserves it with blank entries. */
  const legendOf = (series: number) => (series > 1 ? Array.from({ length: series }, (_, index) => ({ label: '\u00a0', colour: ((index % 6) + 1) as SeriesColour })) : []);
</script>

<!-- The chart grid with no lines (D-3): the same cells as `MetricsSection`, a title line over a chart box each. -->
<div class="chart-grid" aria-hidden="true">
  {#each SHAPE as series, cell (cell)}
    <div class="cell">
      <p class="chart-title"></p>
      <ChartSkeleton {span} axis="percent" format={formatPercent} threshold={DEGRADED_ABOVE} legend={legendOf(series)} />
    </div>
  {/each}
</div>

<style>
  .cell {
    min-inline-size: 0;
  }
</style>
