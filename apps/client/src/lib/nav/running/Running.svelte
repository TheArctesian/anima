<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** The rows (`RunningRow`): builds in progress, live sessions. */
    children: Snippet;
  }

  const { children }: Props = $props();
</script>

<!-- What is running now, at the foot of the bar above the hub status (UR-7 P2): compact rows on the bar's own ground,
     with no material of their own (MAT-009). The bar renders this only when there is something to show. -->
<section class="running" aria-label="Running now">
  {@render children()}
</section>

<style>
  .running {
    display: flex;
    flex-direction: column;
    gap: var(--border-focus);
    padding-block-end: var(--space-2);
  }

  /* An empty slot takes no room. */
  .running:empty {
    display: none;
  }
</style>
