<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import { barRow, type Bar, type BarReadings } from '$lib/metrics/bars/bars';
  import type { BatteryBar } from '$lib/metrics/battery/battery';
  import { containerRow } from '$lib/metrics/container/container';
  import { motionPreference } from '$lib/motion/preference';
  import { crossFade } from '$lib/motion/roles/transitions';

  interface Props {
    readings: BarReadings | null;
    source: MetricsSource | null | undefined;
    /** The list's form: the value over its bar in a fixed slot, the words in the column head (row H of the site language plan). */
    compact?: boolean;
    /** The battery of a laptop, a phone or a tablet; `null` for a device with none (MET-032). */
    battery?: BatteryBar | null;
    /** The hub's own device: CPU and memory of its container, the memory as used of its limit, no disk (HMD-073). */
    container?: boolean;
    /** The bytes of memory in use of the container; `null` before they are known. */
    memoryBytes?: number | null;
  }

  const { readings, source, compact = false, battery = null, container = false, memoryBytes = null }: Props = $props();
  const preference = motionPreference();
  const row = $derived(container ? containerRow(readings, source, memoryBytes) : barRow(readings, source, battery));
  /** Carbon has no RAM or GPU icon; these are the closest (D18). */
  const ICONS: Record<Bar['key'], IconName> = {
    cpu: 'carbon:chip',
    memory: 'carbon:sim-card',
    disk: 'carbon:vmdk-disk',
    vram: 'carbon:spyre-accelerator',
    battery: 'carbon:battery-half',
  };
  const WORDS: Record<Bar['key'], string> = { cpu: 'CPU', memory: 'Memory', disk: 'Disk', vram: 'VRAM', battery: 'Battery' };

  /** The battery's shape says charging or low, so neither rests on colour alone (MET-032). */
  function iconOf(bar: Bar): IconName {
    if (battery === null || bar.key !== 'battery') return ICONS[bar.key];
    return battery.charging ? 'carbon:battery-charging' : battery.low ? 'carbon:battery-low' : ICONS.battery;
  }
</script>

<!-- CPU, memory, disk and, for a device with a GPU, its memory: each an icon and a word over its value and a thin
     bar (UR-7, D18). Over 85 % the icon and value take the warning colour, and the label says so (MET-004). -->
{#if row.kind === 'empty'}
  <span class="none">{row.text}</span>
{:else}
  <span class="meters" class:compact class:container class:reduced={preference.reduced}>
    {#each row.bars as bar (bar.key)}
      <span
        class="meter {bar.key}"
        class:degraded={bar.degraded}
        role={bar.percent === null ? undefined : 'meter'}
        aria-label={bar.percent === null ? undefined : bar.label}
        aria-valuemin={bar.percent === null ? undefined : 0}
        aria-valuemax={bar.percent === null ? undefined : 100}
        aria-valuenow={bar.percent}
        aria-valuetext={bar.percent === null ? undefined : (bar.spoken ?? (bar.degraded ? `${bar.text}, degraded` : bar.text))}
        title={bar.tooltip}
      >
        <span class="word"><Icon name={iconOf(bar)} size="sm" /><span class="name">{WORDS[bar.key]}</span></span>
        <!-- A new value cross-fades over the old one in one cell; it never counts (MET-008, MOT-008). -->
        <span class="slot">
          {#key bar.text}<span class="value" transition:crossFade
              >{#if bar.key === 'battery'}<span class="mark"><Icon name={iconOf(bar)} size="sm" /></span>{/if}{bar.text}</span
            >{/key}
        </span>
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
    grid-template-columns: repeat(auto-fit, minmax(min(100%, var(--space-16)), 1fr));
    gap: var(--space-2);
    inline-size: 100%;
  }

  /* A tile keeps its meters on one line: with a battery and a GPU's memory each meter may be narrower. */
  .meters:has(.battery) {
    grid-template-columns: repeat(auto-fit, minmax(min(100%, var(--space-12)), 1fr));
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

  /* A narrow tile with five meters ends a long word with an ellipsis. */
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* The battery's shape is its only mark of charging in a tile and a row, so it has the colour of text, not the quiet one. */
  .battery .word :global(.icon),
  .mark :global(.icon) {
    color: var(--text-secondary);
  }

  .slot {
    display: grid;
  }

  .value {
    grid-area: 1 / 1;
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

  /*
   * The list: three slots of one width, so each row has one height and the values stand under the words of the column
   * head. The memory of a GPU is not in the list: the tile and the device page show it.
   */
  .compact {
    grid-template-columns: repeat(3, var(--usage-slot));
    gap: var(--space-3);
  }

  .compact .word,
  .compact .vram {
    display: none;
  }

  /* The battery is a fourth slot; the list's head has a word for it only when a device in the list has one. */
  .compact:has(.battery) {
    grid-template-columns: repeat(4, var(--usage-slot));
  }

  /* The list has no word over each value: the battery's own shape stands before its value there. */
  .mark {
    display: none;
  }

  /* The hub's memory, "0.8 of 2.0 GiB", takes the slots of the memory and of the disk, which the hub has none of (HMD-072). */
  .compact.container .memory {
    grid-column: span 2;
  }

  /* "310 of 512 MiB" stays on one line, so the row keeps its one height (MOT-007). */
  .container .memory .value {
    white-space: nowrap;
  }

  .compact .battery .value {
    white-space: nowrap;
  }

  .compact .mark {
    display: inline-flex;
    margin-inline-end: var(--space-1);
    max-block-size: 1lh;
    vertical-align: middle;
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

  /* The OS or the Settings choice (MOT-043): the bar is at its new length at once. */
  .reduced .fill {
    transition: none;
  }
</style>
