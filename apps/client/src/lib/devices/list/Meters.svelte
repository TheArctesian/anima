<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import { barRow, type Bar, type BarReadings } from '$lib/metrics/bars/bars';

  interface Props {
    readings: BarReadings | null;
    source: MetricsSource | null | undefined;
  }

  const { readings, source }: Props = $props();
  const row = $derived(barRow(readings, source));
  /** Carbon has no RAM or GPU icon; these are the closest (D18). */
  const ICONS: Record<Bar['key'], IconName> = {
    cpu: 'carbon:chip',
    memory: 'carbon:sim-card',
    disk: 'carbon:vmdk-disk',
    vram: 'carbon:spyre-accelerator',
  };
  const WORDS: Record<Bar['key'], string> = { cpu: 'CPU', memory: 'Memory', disk: 'Disk', vram: 'VRAM' };
</script>

<!-- CPU, memory, disk and, for a device with a GPU, its memory: each an icon and a word over its value and a thin
     bar (UR-7, D18). Over 85 % the icon and value take the warning colour, and the label says so (MET-004). -->
{#if row.kind === 'empty'}
  <span class="none">{row.text}</span>
{:else}
  <span class="meters" style:--meters={row.bars.length}>
    {#each row.bars as bar (bar.key)}
      <span
        class="meter"
        class:degraded={bar.degraded}
        role={bar.percent === null ? undefined : 'meter'}
        aria-label={bar.percent === null ? undefined : bar.label}
        aria-valuemin={bar.percent === null ? undefined : 0}
        aria-valuemax={bar.percent === null ? undefined : 100}
        aria-valuenow={bar.percent}
        aria-valuetext={bar.percent === null ? undefined : bar.degraded ? `${bar.text}, degraded` : bar.text}
        title={bar.tooltip}
      >
        <span class="word"><Icon name={ICONS[bar.key]} size="sm" />{WORDS[bar.key]}</span>
        <span class="value">{bar.text}</span>
        <span class="track" aria-hidden="true"><span class="fill" style:--fill={(bar.percent ?? 0) / 100}></span></span>
      </span>
    {/each}
  </span>
{/if}

<style>
  .none {
    color: var(--text-quiet);
    font-size: var(--text-sm);
  }

  .meters {
    display: grid;
    grid-template-columns: repeat(var(--meters), minmax(0, 1fr));
    gap: var(--space-3);
    inline-size: 100%;
  }

  .meter {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .word {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    overflow: hidden;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .word :global(.icon) {
    flex: none;
    color: var(--text-quiet);
  }

  .value {
    font-variant-numeric: tabular-nums;
    line-height: var(--leading-tight);
  }

  .track {
    display: block;
    block-size: var(--border-focus);
    background: var(--border-subtle);
    overflow: hidden;
  }

  /* Only transform moves (MOT-006). */
  .fill {
    display: block;
    block-size: 100%;
    background: var(--chart-series-2);
    transform: scaleX(var(--fill));
    transform-origin: left center;
    transition: transform var(--duration-base) var(--ease-enter);
  }

  .degraded {
    color: var(--status-warning-text);
  }

  .degraded .word :global(.icon) {
    color: var(--status-warning);
  }

  .degraded .fill {
    background: var(--status-warning);
  }

  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
