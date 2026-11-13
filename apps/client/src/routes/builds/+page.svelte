<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import BuildsView from '$lib/builds/view/BuildsView.svelte';
  import { capabilities } from '$lib/platform/platform';

  // Builds is a desktop app view: the hub does not offer a builder to a browser (BLD-080).
  const { builds } = capabilities();
  $effect(() => {
    if (!builds) void goto(resolve('/devices'), { replaceState: true });
  });
</script>

{#if builds}
  <BuildsView />
{/if}
