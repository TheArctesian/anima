<script lang="ts">
  import type { Snippet } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { useExtensions } from '$lib/extensions/store.svelte';

  // Guard only: while Projects is off the page does not show (EXT-002). The Projects page itself is another task's.
  const { children }: { children: Snippet } = $props();
  const extensions = useExtensions();
  const on = $derived(extensions.isOn('projects'));
  $effect(() => {
    if (extensions.status !== 'loading' && !on) void goto(resolve('/devices'), { replaceState: true });
  });
</script>

{#if on}
  {@render children()}
{/if}
