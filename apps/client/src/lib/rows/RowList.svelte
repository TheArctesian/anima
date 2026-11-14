<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** What the list holds, for assistive technology when no heading names it. */
    label?: string | undefined;
    /** The rows: each a `Row`. */
    children: Snippet;
  }

  const { label, children }: Props = $props();
</script>

<!-- Hairline rows (UR-7, D9): no card; a hairline above the first row and under each. -->
<ul aria-label={label}>{@render children()}</ul>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }
</style>
