<script lang="ts">
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import type { MetricRange } from '$lib/metrics/api';
  import '$lib/metrics/bars/geometry.css';
  import './geometry.css';
  import MetricsSkeleton from './MetricsSkeleton.svelte';
  import { hostKeyText, type OverviewDevice } from './overview';

  interface Props {
    range: MetricRange;
    source: MetricsSource;
    now: () => Date;
    /** The device's SSH facts when it is known; `null` reserves the longest case, a host key. */
    ssh: OverviewDevice['ssh'] | null;
  }

  const { range, source, now, ssh }: Props = $props();
  /** Status, maintenance, three uptimes, checks and the host key: the facts of `OverviewPanel`. */
  /** What a key looks like when the device is not known: algorithm, then "SHA256:" and 43 base64 characters. */
  const REPRESENTATIVE_KEY = `ssh-ed25519 SHA256:${'x'.repeat(43)}`;
  const keyText = $derived(ssh === null ? REPRESENTATIVE_KEY : hostKeyText(ssh));
  const FACTS = Array.from({ length: 7 }, (_, index) => index);
</script>

<!-- The host key row holds the loaded row's own text, hidden, in the same `.key` styling, so it wraps to the same lines at every width (MOT-007). -->
<!-- The loaded Overview's top part with no text (D-9): the facts, then the chart grid. Real `dt` and `dd` lines of a non-breaking space give the rows their height. -->
<dl class="overview-facts" aria-hidden="true">
  {#each FACTS as fact (fact)}
    <div>
      <dt>&nbsp;</dt>
      {#if fact === 6}
        <dd class="key hidden">{keyText}</dd>
      {:else}
        <dd>&nbsp;</dd>
      {/if}
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
  .hidden {
    visibility: hidden;
  }

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
