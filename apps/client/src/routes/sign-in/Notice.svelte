<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';

  const { icon, children }: { icon: IconName; children: Snippet } = $props();
</script>

<div class="notice">
  <span class="mark"><Icon name={icon} /></span>
  <div class="body">{@render children()}</div>
</div>

<style>
  .notice {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--status-down-tint);
    font-size: var(--text-md);
    line-height: var(--leading-normal);
  }

  .mark {
    color: var(--status-down-text);
    line-height: var(--leading-normal);
  }

  .body {
    display: grid;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  .body :global(p) {
    margin: 0;
  }
</style>
