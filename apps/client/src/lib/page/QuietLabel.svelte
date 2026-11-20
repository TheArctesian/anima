<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
  }

  const { children }: Props = $props();
</script>

<!-- A quiet label in sentence case (UR-7, D6): column heads, an eyebrow over a title, the name of a group of rows. -->
<span class="label">{@render children()}</span>

<style>
  .label {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
    line-height: var(--leading-tight);
  }
</style>
