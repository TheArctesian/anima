<script lang="ts">
  import type { Snippet } from 'svelte';
  import { crossFade } from '$lib/motion/roles/transitions';

  interface Props {
    /** What the reading shows; a new value cross-fades over the old one. */
    value: string | number | boolean;
    children: Snippet;
  }

  const { value, children }: Props = $props();
</script>

<!-- MOT-008: a reading that changes cross-fades in place; it never counts through values that never happened. -->
<span class="reading">
  {#key value}
    <span class="value" transition:crossFade>{@render children()}</span>
  {/key}
</span>

<style>
  .reading {
    display: inline-grid;
    justify-items: inherit;
  }

  /* Old and new share one cell, so the fade moves nothing around them (MOT-007). */
  .value {
    grid-area: 1 / 1;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    white-space: nowrap;
  }
</style>
