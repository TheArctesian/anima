<script lang="ts">
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import { motionPreference } from '$lib/motion/preference';
  import { crossFade } from '$lib/motion/roles/transitions';
  import './geometry.css';
  import { barRow, type BarReadings } from './bars';

  interface Props {
    readings: BarReadings | null;
    source: MetricsSource | null | undefined;
  }

  const { readings, source }: Props = $props();
  const preference = motionPreference();
  const row = $derived(barRow(readings, source));
</script>

<!-- One block size for the bars and the empty state, so the row never changes size (MOT-007). -->
<div class="row" class:empty={row.kind === 'empty'} class:reduced={preference.reduced}>
  {#if row.kind === 'empty'}
    <p>{row.text}</p>
  {:else}
    {#each row.bars as bar (bar.key)}
      <div
        class="bar"
        class:degraded={bar.degraded}
        role={bar.percent === null ? undefined : 'meter'}
        aria-label={bar.percent === null ? undefined : bar.label}
        aria-valuemin={bar.percent === null ? undefined : 0}
        aria-valuemax={bar.percent === null ? undefined : 100}
        aria-valuenow={bar.percent}
        aria-valuetext={bar.percent === null ? undefined : bar.degraded ? `${bar.text}, degraded` : bar.text}
        title={bar.tooltip}
      >
        <span class="label"
          >{bar.label}{#if bar.note !== null}<span class="note">{` ${bar.note}`}</span>{/if}{#if bar.degraded}<span class="warn" aria-hidden="true"> ▲</span>{/if}</span
        >
        <!-- A new value cross-fades over the old one; it never counts (MET-008, MOT-008). -->
        <span class="slot">
          {#key bar.text}
            <span class="value" transition:crossFade>{bar.text}</span>
          {/key}
        </span>
        <span class="track" aria-hidden="true">
          <span class="fill" style:--fill={(bar.percent ?? 0) / 100}></span>
        </span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .row {
    box-sizing: border-box;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
    min-inline-size: 0;
    block-size: var(--metric-bars-block-size);
    overflow: hidden;
  }

  .row.empty {
    grid-template-columns: minmax(0, 1fr);
    align-items: center;
  }

  p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .bar {
    min-inline-size: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-rows: var(--metric-bars-label-block-size) var(--space-1);
    column-gap: var(--space-2);
    row-gap: var(--space-1);
    font-size: var(--text-xs);
    line-height: var(--metric-bars-label-block-size);
  }

  .label {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* A fixed width: the widest value fits, so a new one moves nothing (MOT-007). */
  .slot {
    display: inline-grid;
    justify-items: end;
    min-inline-size: calc(var(--space-8) + var(--space-1));
  }

  .value {
    grid-area: 1 / 1;
    color: var(--text);
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .track {
    grid-column: 1 / -1;
    border-radius: var(--radius-sm);
    background: var(--surface-overlay);
    overflow: hidden;
  }

  /* Only transform moves (MOT-006); instant under reduced motion. */
  .fill {
    display: block;
    block-size: 100%;
    background: var(--accent);
    transform: scaleX(var(--fill));
    transform-origin: left center;
    transition: transform var(--duration-base) var(--ease-enter);
  }

  .reduced .fill {
    transition: none;
  }

  /* Degraded is also a marker and a bold value, not colour alone (MET-004); the value slot keeps its width. */
  .degraded .value {
    font-weight: var(--weight-bold);
  }

  .warn {
    color: var(--status-degraded);
  }

  /* "+3 drives": the other drives of the device, quieter than the label (MET-025). */
  .note {
    color: var(--text-tertiary);
  }

  .degraded .fill {
    background: var(--status-degraded);
  }
</style>
