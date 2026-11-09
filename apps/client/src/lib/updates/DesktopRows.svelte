<script lang="ts">
  import BuildsRow from '$lib/builds/BuildsRow.svelte';
  import { keyStoreAvailable } from '$lib/desktop/token';
  import LogFolderRow from '$lib/process-log/LogFolderRow.svelte';
  import UpdatesRow from './UpdatesRow.svelte';
</script>

<!-- The desktop app alone updates itself, keeps a process log and builds releases; the web and iOS clients show none of them (UPD-006, LOG-008, BLD-020). -->
{#if keyStoreAvailable()}
  <UpdatesRow />
  <LogFolderRow />
  <BuildsRow />
{/if}
