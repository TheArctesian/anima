<script lang="ts">
  import { keepFresh, threadsStore } from './watch.svelte';

  const store = threadsStore();

  /** The bar is on each page, so the count is fresh on each page (THR-057). */
  $effect(() => keepFresh(store));
</script>

<!-- How many threads need the person, on the item Threads of the bar; nothing when none does. -->
{#if store.needsYou > 0}
  <span class="count" aria-label={store.needsYou === 1 ? '1 thread needs you' : `${String(store.needsYou)} threads need you`}>{store.needsYou}</span>
{/if}

<style>
  .count {
    margin-inline-start: auto;
    color: var(--status-degraded-text);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
    font-weight: var(--weight-medium);
  }
</style>
