<script lang="ts">
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import Icon from '$lib/icons/Icon.svelte';
  import type { MetricRange } from '$lib/metrics/api';
  import '$lib/metrics/bars/geometry.css';
  import './geometry.css';
  import OverviewFacts from './facts/OverviewFacts.svelte';
  import MetricsSkeleton from './MetricsSkeleton.svelte';
  import type { OverviewDevice } from './overview';

  interface Props {
    range: MetricRange;
    source: MetricsSource;
    now: () => Date;
    /** The device's SSH facts when it is known; `null` reserves the longest case, a host key. */
    ssh: OverviewDevice['ssh'] | null;
  }

  const { range, source, now, ssh }: Props = $props();
  /** What a key looks like when the device is not known: algorithm, then "SHA256:" and 43 base64 characters. */
  const REPRESENTATIVE_KEY = { algorithm: 'ssh-ed25519', fingerprintSha256: `SHA256:${'x'.repeat(43)}`, firstSeenAt: '' };
  /** The facts cards with values of the widest text; the text is hidden and only the cards show. */
  const shape = $derived({
    status: 'UNKNOWN' as const,
    maintenance: false,
    day: null,
    week: null,
    month: null,
    ssh: ssh ?? { enabled: true, hostKey: REPRESENTATIVE_KEY },
  });
</script>

<!--
  The loaded Overview with no text (D-9): the same facts cards, drawn by the same component with each card's content
  hidden, so a host key wraps to the same lines at every width; then the Metrics card (MOT-007).
-->
<div class="ghost" aria-hidden="true" inert>
  <OverviewFacts device={shape} checks={[]} />
</div>
<div class="overview-card metrics" aria-hidden="true">
  <div class="head" class:ssh={source === 'SSH'}><div class="overview-card-title">&nbsp;</div></div>
  {#if source === 'SSH'}
    <div class="bars"></div>
    <MetricsSkeleton {range} {now} />
  {:else}
    <!-- The block the loaded section has when metrics are off, with its icon, sentence and button hidden. -->
    <div class="metrics-off ghost-text">
      <span class="off-icon"><Icon name="nf-md-chart_line" size="lg" /></span>
      <p>&nbsp;</p>
      <span class="settings"></span>
    </div>
  {/if}
</div>

<style>
  /* Each card keeps its box; what it says is not drawn. */
  .ghost :global(.overview-card > *),
  .ghost-text > * {
    visibility: hidden;
  }

  /* The gaps and heights of `MetricsSection`: its head holds the range buttons, then the bars row. */
  .metrics {
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

  .off-icon {
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
  }

  .settings {
    block-size: var(--target-min-touch);
  }
</style>
