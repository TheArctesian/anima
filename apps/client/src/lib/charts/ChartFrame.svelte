<script lang="ts">
  import type { Snippet } from 'svelte';
  import { thresholdY, type SeriesColour } from './plot';
  import { PLOT, type TimeWindow, type ValueScale } from './scales';
  import { timeTicks, valueTicks } from './ticks';

  interface Props {
    window: TimeWindow;
    scale: ValueScale;
    /** The text of a value label; without it the labels keep their place but show nothing (a skeleton's open axis). */
    format?: (value: number) => string;
    threshold?: number;
    /** Whether `window` is in milliseconds and gets time labels. */
    timeAxis: boolean;
    /** Shown when there is more than one series, so colour is never the only cue. */
    legend: readonly { readonly label: string; readonly colour: SeriesColour }[];
    /** The text alternative; the plot is then an image with this label. Without it the whole frame is hidden (a skeleton). */
    label?: string;
    /** Drawn over the grid, in a box of the plot's size. */
    children?: Snippet;
  }

  const { window, scale, format, threshold, timeAxis, legend, label, children }: Props = $props();
  const values = $derived(valueTicks(scale));
  const thresholdAt = $derived(thresholdY(scale, threshold));
  // The zone is read here, at the edge; the tick rule takes it as a parameter.
  const times = $derived(timeAxis ? timeTicks(window, -new Date(window.end).getTimezoneOffset() * 60_000) : []);
  const percentOf = (part: number, whole: number) => `${String((part / whole) * 100)}%`;
</script>

<!-- The box, grid, axes and legend; only `children` differs between a chart and its skeleton (MOT-007). -->
<div class="frame" aria-hidden={label === undefined ? 'true' : undefined}>
  <div class="values" aria-hidden="true">
    {#each values as tick (tick.value)}
      <span style:inset-block-start={percentOf(tick.y, PLOT.height)}>{format?.(tick.value) ?? ''}</span>
    {/each}
  </div>
  <div class="plot">
    <svg viewBox="0 0 {PLOT.width} {PLOT.height}" preserveAspectRatio="none" role={label === undefined ? undefined : 'img'} aria-label={label}>
      {#each values as tick (tick.value)}
        <line class="grid" x1="0" x2={PLOT.width} y1={tick.y} y2={tick.y} />
      {/each}
      {#if thresholdAt !== null}
        <line class="threshold" x1="0" x2={PLOT.width} y1={thresholdAt} y2={thresholdAt} />
      {/if}
    </svg>
    {@render children?.()}
  </div>
  {#if timeAxis}
    <div class="times" aria-hidden="true">
      {#each times as tick (tick.at)}
        <span style:inset-inline-start={percentOf(tick.x, PLOT.width)}>{tick.text}</span>
      {/each}
    </div>
  {/if}
  {#if legend.length > 1}
    <!-- The label names each series with its value, so the legend is for the eye only. -->
    <ul class="legend" aria-hidden="true">
      {#each legend as entry, index (index)}
        <li style:--series="var(--chart-series-{entry.colour})">{entry.label}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  /* Every size comes from tokens and none from the data: the chart and its skeleton are one box (MOT-007). */
  .frame {
    display: grid;
    grid-template-columns: var(--space-16) minmax(0, 1fr);
    column-gap: var(--space-2);
    row-gap: var(--space-1);
    padding-block-start: var(--space-2);
    color: var(--chart-axis);
    font-size: var(--text-xs);
    line-height: var(--leading-normal);
    font-variant-numeric: tabular-nums;
  }

  .values,
  .times {
    position: relative;
  }

  .values span {
    position: absolute;
    inset-inline-end: 0;
    translate: 0 -50%;
    white-space: nowrap;
  }

  .plot {
    position: relative;
    grid-column: 2;
    block-size: calc(var(--space-16) * 2);
    overflow: hidden;
  }

  .plot svg {
    position: absolute;
    inset: 0;
    inline-size: 100%;
    block-size: 100%;
  }

  line {
    stroke-width: var(--border-thin);
    vector-effect: non-scaling-stroke;
  }

  .grid {
    stroke: var(--chart-grid);
  }

  .threshold {
    stroke: var(--chart-threshold);
  }

  .times {
    grid-column: 2;
    block-size: calc(var(--text-xs) * var(--leading-normal));
  }

  .times span {
    position: absolute;
    translate: -50% 0;
    white-space: nowrap;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    grid-column: 2;
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .legend li::before {
    content: '';
    inline-size: var(--space-3);
    block-size: var(--border-focus);
    background: var(--series);
  }
</style>
