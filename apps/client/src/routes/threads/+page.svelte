<script lang="ts">
  import { ThreadsStore } from '$lib/agent-threads/store.svelte';
  import ThreadsScreen from '$lib/agent-threads/ThreadsScreen.svelte';
  import { desktopThreadsShell } from '$lib/desktop/agent-threads/shell';
  import { capabilities } from '$lib/platform/platform';
  import EmptyState from '$lib/states/EmptyState.svelte';

  const here = capabilities().agentThreads;
  const store = new ThreadsStore(desktopThreadsShell);

  /** The list is read when the page opens; only the desktop application can read the files of its computer. */
  $effect(() => {
    if (here) void store.loadList();
  });
</script>

{#if here}
  <ThreadsScreen {store} />
{:else}
  <EmptyState title="Threads are in the desktop application" line="This page reads the sessions of Claude Code and Codex on the computer where the application runs." />
{/if}
