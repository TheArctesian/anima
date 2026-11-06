<script lang="ts">
  import type { Snippet } from 'svelte';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { tabIds, type TabName } from './tabs';

  interface Props {
    name: TabName;
    /** The panel's first data is on its way. */
    loading: boolean;
    clock?: LoadingClock;
    children: Snippet;
  }

  const { name, loading, clock, children }: Props = $props();
  const ids = $derived(tabIds(name));
  // The clock is fixed for the life of the panel.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);

  /** Placeholder rows of the list rows' height; static, so reduced motion needs no change. */
  const ROWS = Array.from({ length: 4 }, (_, index) => index);
</script>

<div class="panel" role="tabpanel" id={ids.panel} aria-labelledby={ids.tab} tabindex="0" aria-busy={loading}>
  {#if skeleton.visible}
    <ul class="skeleton" aria-hidden="true" transition:crossFade>
      {#each ROWS as row (row)}<li></li>{/each}
    </ul>
  {:else if !loading}
    <div class="content" transition:crossFade>
      {@render children()}
    </div>
  {/if}
</div>

<style>
  /* The skeleton and the content share one cell, so the cross-fade moves nothing (MOT-007). */
  .panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-inline-size: 0;
  }

  .panel:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .skeleton,
  .content {
    grid-area: 1 / 1;
    min-inline-size: 0;
  }

  .skeleton {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .skeleton li {
    box-sizing: border-box;
    block-size: var(--target-min-touch);
    border-block-end: var(--border-thin) solid var(--border);
    background: var(--surface);
  }
</style>
