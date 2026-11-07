<script lang="ts">
  import { untrack } from 'svelte';
  import ChartSkeleton from '$lib/charts/ChartSkeleton.svelte';
  import type { MetricRange } from '$lib/metrics/api';
  import { DEGRADED_ABOVE } from '$lib/metrics/bars/bars';
  import { rangeWindow } from '$lib/metrics/ranges';
  import { formatPercent } from '$lib/metrics/units';
  import './geometry.css';

  interface Props {
    range: MetricRange;
    now: () => Date;
  }

  const { range, now }: Props = $props();
  // The window is fixed when the range is: a skeleton is not redrawn with the clock.
  const span = $derived(rangeWindow(range, untrack(now)));
  /** CPU, memory, disk and network: the charts every SSH device has. */
  const CELLS = Array.from({ length: 4 }, (_, index) => index);
</script>

<!-- The chart grid with no lines (D-3): the same cells as `MetricsSection`, a title line over a chart box each. -->
<div class="chart-grid" aria-hidden="true">
  {#each CELLS as cell (cell)}
    <div class="cell">
      <p class="chart-title"></p>
      <ChartSkeleton {span} axis="percent" format={formatPercent} threshold={DEGRADED_ABOVE} />
    </div>
  {/each}
</div>

<style>
  .cell {
    min-inline-size: 0;
  }
</style>
