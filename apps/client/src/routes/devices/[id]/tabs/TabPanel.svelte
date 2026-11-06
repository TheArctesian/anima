<script lang="ts">
  import { untrack, type Snippet } from 'svelte';
  import { browserClock, skeletonGate, type LoadingClock } from '$lib/devices/screen/loading/loading';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { tabIds, type TabName } from './tabs';

  interface Props {
    name: TabName;
    /** The panel's first data is on its way. */
    loading: boolean;
    clock?: LoadingClock;
    children: Snippet;
  }

  const { name, loading, clock = browserClock, children }: Props = $props();
  const ids = $derived(tabIds(name));
  let skeleton = $state(false);
  const gate = skeletonGate(untrack(() => clock), (visible) => {
    skeleton = visible;
  });

  /** Loading (5.7.3): a skeleton only after `threshold-loading`, kept `threshold-loading-min`, then the data cross-fades in. */
  $effect(() => {
    const busy = loading;
    untrack(() => {
      if (busy) gate.begin();
      else gate.end();
    });
  });

  $effect(() => () => {
    gate.stop();
  });

  /** Placeholder rows of the list rows' height; static, so reduced motion needs no change. */
  const ROWS = Array.from({ length: 4 }, (_, index) => index);
</script>

<div class="panel" role="tabpanel" id={ids.panel} aria-labelledby={ids.tab} tabindex="0" aria-busy={loading}>
  {#if skeleton}
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
