<script lang="ts">
  import BuildsRow from '$lib/builds/BuildsRow.svelte';
  import { capabilities } from '$lib/platform/platform';
  import LogFolderRow from '$lib/process-log/LogFolderRow.svelte';
  import CommandLineRow from './CommandLineRow.svelte';
  import UpdatesRow from './UpdatesRow.svelte';

  const { updates, commandLine, processLog, builds } = capabilities();
</script>

<!-- The desktop app alone updates itself, puts its command line on the PATH, keeps a process log and builds releases; the web and iOS clients show none of them (UPD-006, INS-006, LOG-008, BLD-020). -->
{#if updates}
  <UpdatesRow />
{/if}
{#if commandLine}
  <CommandLineRow />
{/if}
{#if processLog}
  <LogFolderRow />
{/if}
{#if builds}
  <BuildsRow />
{/if}
