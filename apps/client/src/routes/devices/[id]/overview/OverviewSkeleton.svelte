<script lang="ts">
  import type { MetricRange } from '$lib/metrics/api';
  import '$lib/metrics/bars/geometry.css';
  import './geometry.css';
  import MetricsSkeleton from './MetricsSkeleton.svelte';

  interface Props {
    range: MetricRange;
    now: () => Date;
  }

  const { range, now }: Props = $props();
  /** Status, maintenance, three uptimes, checks and the host key: the facts of `OverviewPanel`. */
  const FACTS = Array.from({ length: 7 }, (_, index) => index);
</script>

<!-- The loaded Overview's top part with no text (D-9): the facts, then the chart grid. Real `dt` and `dd` lines of a non-breaking space give the rows their height. -->
<dl class="overview-facts" aria-hidden="true">
  {#each FACTS as fact (fact)}
    <div>
      <dt>&nbsp;</dt>
      <dd class:key={fact === 6}>&nbsp;</dd>
    </div>
  {/each}
</dl>
<div class="metrics">
  <div class="head"><div class="title">&nbsp;</div></div>
  <div class="bars"></div>
  <MetricsSkeleton {range} {now} />
</div>

<style>
  /* The gaps and heights of `MetricsSection`: its head holds the range buttons, then the bars row. */
  .metrics {
    display: grid;
    gap: var(--space-4);
  }

  .head {
    display: flex;
    align-items: center;
    min-block-size: var(--target-min-desktop);
  }

  .bars {
    block-size: var(--metric-bars-block-size);
  }

  .title {
    margin: 0;
    font-size: var(--text-lg);
  }
</style>
