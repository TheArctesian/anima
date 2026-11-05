<script lang="ts">
  import SignOutRow from '$lib/access/SignOutRow.svelte';
  import { keyStoreAvailable } from '$lib/desktop/token';
  import LogFolderRow from '$lib/process-log/LogFolderRow.svelte';
  import UpdatesRow from '$lib/updates/UpdatesRow.svelte';
</script>

<h1>Settings</h1>

<SignOutRow />

<!-- The desktop app alone updates itself and keeps a process log (UPD-006, LOG-008). -->
{#if keyStoreAvailable()}
  <UpdatesRow />
  <LogFolderRow />
{/if}

<style>
  /* The title's line height is reserved from tokens, so the font load cannot move it (WP-11 H-7, R-042). */
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-2xl) * var(--leading-tight));
  }
</style>
