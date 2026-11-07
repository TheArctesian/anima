<script lang="ts">
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import type { MetricRange } from '$lib/metrics/api';
  import '$lib/metrics/bars/geometry.css';
  import './geometry.css';
  import MetricsSkeleton from './MetricsSkeleton.svelte';

  interface Props {
    range: MetricRange;
    source: MetricsSource;
    now: () => Date;
  }

  const { range, source, now }: Props = $props();
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
  <div class="head" class:ssh={source === 'SSH'}><div class="metrics-title">&nbsp;</div></div>
  {#if source === 'SSH'}
    <div class="bars"></div>
    <MetricsSkeleton {range} {now} />
  {:else}
    <!-- The one line the loaded section has when metrics are off. -->
    <p class="metrics-off">&nbsp;</p>
  {/if}
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
  }

  .ssh {
    min-block-size: var(--target-min-desktop);
  }

  .bars {
    block-size: var(--metric-bars-block-size);
  }
</style>
