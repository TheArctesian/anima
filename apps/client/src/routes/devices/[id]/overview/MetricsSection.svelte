<script lang="ts">
  import LineChart from '$lib/charts/LineChart.svelte';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import { chartsOf } from '$lib/metrics/charts';
    import type { MetricRange } from '$lib/metrics/api';
  import MetricBars from '$lib/metrics/bars/MetricBars.svelte';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { RANGES, rangeGap } from '$lib/metrics/ranges';
  import './geometry.css';
  import type { DeviceMetrics } from './metrics.svelte';
  import MetricsSkeleton from './MetricsSkeleton.svelte';

  interface Props {
    source: MetricsSource;
    metrics: Pick<DeviceMetrics, 'view' | 'span' | 'shownRange' | 'readings' | 'error' | 'liveError'>;
    range: MetricRange;
    now: () => Date;
    clock?: LoadingClock;
    onRange: (range: MetricRange) => void;
    onRetry: () => void;
    onSettings: () => void;
  }

  const { source, metrics, range, now, clock, onRange, onRetry, onSettings }: Props = $props();
  const charts = $derived(metrics.view === null ? [] : chartsOf(metrics.view.series, metrics.view.gpus));
  // The old chart stays while a switch is on its way, so its label and gap follow the data on screen, not the button.
  const drawn = $derived(metrics.shownRange ?? range);
  const phrase = $derived(RANGES.find((each) => each.range === drawn)?.phrase ?? '');
  const waiting = $derived(metrics.view === null && metrics.error === null);
  // The clock is fixed for the life of the section.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => waiting, clock);
</script>

<section class="metrics" aria-labelledby="overview-metrics">
  <div class="head">
    <h2 id="overview-metrics" class="metrics-title">Metrics</h2>
    {#if source === 'SSH'}
      <div class="range" role="group" aria-label="Time range">
        {#each RANGES as choice (choice.range)}
          <button type="button" aria-pressed={choice.range === range} onclick={() => { onRange(choice.range); }} {@attach press()}>
            {choice.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>
  {#if source !== 'SSH'}
    <p class="metrics-off">
      Metrics are off for this device.
      <button type="button" class="link" onclick={onSettings} {@attach press()}>Open Settings</button>
    </p>
  {:else}
    <MetricBars readings={metrics.readings} {source} />
    {#if metrics.liveError !== null}
      <p class="live" role="status">{metrics.liveError.message}</p>
    {/if}
    <!-- The skeleton and the charts share one cell, so the cross-fade moves nothing (MOT-007). -->
    <div class="area">
      {#if metrics.error !== null}
        <div class="notice" role="alert" transition:crossFade>
          <p>{metrics.error.message}</p>
          <button type="button" onclick={onRetry} {@attach press()}>Retry</button>
        </div>
      {:else if skeleton.visible}
        <div class="layer" transition:crossFade><MetricsSkeleton {range} {now} /></div>
      {:else if !waiting}
        <div class="layer" transition:crossFade>
          {#if charts.length === 0 || metrics.span === null}
            <p class="waiting">Waiting for metrics</p>
          {:else}
            <div class="chart-grid">
              {#each charts as chart (chart.key)}
                <div class="cell">
                  <h3 class="chart-title">{chart.title}</h3>
                  <LineChart
                    title={`${chart.title}, ${phrase}`}
                    gap={rangeGap(drawn)}
                    series={chart.series}
                    span={metrics.span}
                    axis={chart.axis}
                    format={chart.format}
                    threshold={chart.threshold}
                  />
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .metrics {
    display: grid;
    gap: var(--space-4);
    min-inline-size: 0;
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .range {
    display: flex;
    gap: var(--space-1);
  }

  .area {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-inline-size: 0;
  }

  .layer,
  .notice {
    grid-area: 1 / 1;
    min-inline-size: 0;
  }

  .cell {
    min-inline-size: 0;
  }

  p {
    margin: 0;
    color: var(--text);
  }

  .waiting,
  .live {
    color: var(--text-muted);
  }

  .notice p {
    margin-block-end: var(--space-2);
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  /* The chosen range has a colour, an underline and, for more contrast, a ring: never colour alone. */
  button[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--surface);
    text-decoration: underline;
    text-decoration-thickness: var(--border-focus);
    text-underline-offset: var(--space-1);
  }

  @media (prefers-contrast: more) {
    button[aria-pressed='true'] {
      box-shadow: inset 0 0 0 var(--border-focus) currentcolor;
    }
  }

  button.link {
    padding: 0 var(--space-1);
    border-color: transparent;
    background: none;
    color: var(--accent);
    text-decoration: underline;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
