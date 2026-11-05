<script lang="ts">
  import { keyStoreAvailable } from '$lib/desktop/token';
  import LogFolderRow from '$lib/process-log/LogFolderRow.svelte';
  import UpdatesRow from './UpdatesRow.svelte';
</script>

<!-- The desktop app alone updates itself and keeps a process log; the web and iOS clients show neither (UPD-006, LOG-008). -->
{#if keyStoreAvailable()}
  <UpdatesRow />
  <LogFolderRow />
{/if}
