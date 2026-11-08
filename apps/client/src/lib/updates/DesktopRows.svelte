<script lang="ts">
  import { capabilities } from '$lib/platform/platform';
  import LogFolderRow from '$lib/process-log/LogFolderRow.svelte';
  import UpdatesRow from './UpdatesRow.svelte';

  const { updates, processLog } = capabilities();
</script>

<!-- The desktop app alone updates itself and keeps a process log; the web and iOS clients show neither (UPD-006, LOG-008). -->
{#if updates}
  <UpdatesRow />
{/if}
{#if processLog}
  <LogFolderRow />
{/if}
