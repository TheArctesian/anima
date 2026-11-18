<script lang="ts">
  import type { LogFilter } from '$lib/events/log';
  import { press } from '$lib/motion/roles/press';
  import { CHIPS, chipFilter, chipOf } from './chips';

  interface Props {
    filter: LogFilter;
    onChange: (filter: LogFilter) => void;
  }

  const { filter, onChange }: Props = $props();
  const on = $derived(chipOf(filter));
</script>

<!-- The log filter in the phone layout (mockup 6): one row of chips that scrolls, one chip on at a time (LOG-009). -->
<div class="phone-log-chips" role="group" aria-label="Show">
  {#each CHIPS as chip (chip.id)}
    <button
      type="button"
      aria-pressed={chip.id === on}
      onclick={() => {
        onChange(chipFilter(chip.id, filter));
      }}
      {@attach press()}>{#if chip.dot !== null}<span class="dot {chip.dot}" aria-hidden="true"></span>{/if}{chip.label}</button
    >
  {/each}
</div>
