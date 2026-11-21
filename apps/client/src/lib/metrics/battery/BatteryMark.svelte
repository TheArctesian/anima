<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import type { BatteryBar } from './battery';

  interface Props {
    battery: BatteryBar;
  }

  const { battery }: Props = $props();
  /** The shape says charging or low, so neither rests on colour alone (MET-032). */
  const icon = $derived<IconName>(battery.charging ? 'carbon:battery-charging' : battery.low ? 'carbon:battery-low' : 'carbon:battery-half');
</script>

<!-- The battery in one line of text where a row shows no meters: its shape, then its charge (MET-032). -->
<span
  class="battery"
  class:low={battery.low}
  role={battery.percent === null ? 'img' : 'meter'}
  aria-label={battery.percent === null ? battery.tooltip : battery.label}
  aria-valuemin={battery.percent === null ? undefined : 0}
  aria-valuemax={battery.percent === null ? undefined : 100}
  aria-valuenow={battery.percent}
  aria-valuetext={battery.percent === null ? undefined : battery.spoken}
  title={battery.tooltip}
>
  <Icon name={icon} size="sm" /><span class="value">{battery.text}</span>
</span>

<style>
  .battery {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
    line-height: var(--leading-tight);
  }

  /* The shape is the only mark of charging here, so it has the colour of text, not the quiet one. */
  .battery :global(.icon) {
    flex: none;
    color: var(--text-secondary);
  }

  .value {
    font-variant-numeric: tabular-nums;
  }

  /* The quiet warning at 20 % and at 10 % while it discharges (MET-032). */
  .low,
  .low :global(.icon) {
    color: var(--status-warning-text);
  }
</style>
