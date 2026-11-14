<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
  }

  const { children }: Props = $props();
</script>

<!-- A small-caps label (UR-7, D6): column heads, an eyebrow over a title, the name of a group of rows. -->
<span class="caps">{@render children()}</span>

<style>
  .caps {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-label);
    line-height: var(--leading-tight);
    text-transform: uppercase;
  }
</style>
