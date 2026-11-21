<script lang="ts">
  import ThreadsScreen from '$lib/agent-threads/ThreadsScreen.svelte';
  import { keepFresh, threadsStore } from '$lib/agent-threads/watch.svelte';
  import { capabilities } from '$lib/platform/platform';
  import EmptyState from '$lib/states/EmptyState.svelte';

  const here = capabilities().agentThreads;
  const store = threadsStore();

  /** The list is read when the page opens and stays fresh while it is seen; only the desktop application can read the files of its computer. */
  $effect(() => (here ? keepFresh(store) : undefined));
</script>

{#if here}
  <ThreadsScreen {store} />
{:else}
  <EmptyState title="Threads are in the desktop application" line="This page reads the sessions of Claude Code and Codex on the computer where the application runs." />
{/if}
