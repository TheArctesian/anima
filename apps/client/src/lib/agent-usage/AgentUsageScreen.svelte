<script lang="ts">
  import { resolve } from '$app/paths';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import Table from '$lib/tables/Table.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import { usageColumns, usageRowKey } from './report/columns';
  import type { UsageQuery } from './query';
  import type { UsageStore } from './store.svelte';
  import UsageFilters from './UsageFilters.svelte';
  import UsageNotes from './UsageNotes.svelte';
  import { summaryLine } from './report/words';

  const REPORTS = 'The daemon of each device reports the use of the AI agent programs.';
  /** The daemon is installed for one device, in its settings; the Devices page is where a device is opened. */
  const WHERE_TO_INSTALL = 'To install it, open a device, then Settings, then Daemon.';
  const NOT_ON_HOSTED = 'A daemon cannot connect to a hub that Anima.ing hosts yet, so this page stays empty on this hub.';

  interface Props {
    store: UsageStore;
    query: UsageQuery;
    onQueryChange: (query: UsageQuery) => void;
    onRetry: () => void;
    /** The hub is one that Anima.ing hosts: the daemon of a computer cannot reach it yet (HM-D16), so no action is offered. */
    hosted?: boolean;
  }

  const { store, query, onQueryChange, onRetry, hosted = false }: Props = $props();
  const view = $derived(store.view);
  const columns = $derived(usageColumns(query.group));
  /** No daemon sent counts, whatever the filter: the whole page is the empty state. */
  const noDaemon = $derived(view !== null && view.everyDevice.length === 0);
</script>

<!-- The use of the AI agent programs (AIS-020): the days and the choices, one line of totals, the table, and under it what the
     numbers are and which devices send none (AIS-021, AIS-024). -->
<section class="usage">
  <PageHeader title="Agent usage" detail="The use of Claude Code and Codex on your devices, from counts only. No text of a session leaves a device." />
  {#if noDaemon}
    {#if hosted}
      <EmptyState title="No use counts yet" line={`${REPORTS} ${NOT_ON_HOSTED}`} />
    {:else}
      <EmptyState title="No use counts yet" line={`${REPORTS} ${WHERE_TO_INSTALL}`}>
        {#snippet action()}
          <a class="action" href={resolve('/devices')}>Open Devices</a>
        {/snippet}
      </EmptyState>
    {/if}
  {:else}
    <UsageFilters {query} devices={view?.everyDevice ?? []} onChange={onQueryChange} />
    <!-- The result keeps the height of a typical page while it loads, so nothing below it moves (MOT-007). -->
    <div class="result" aria-busy={store.status === 'loading'}>
      {#if view === null && store.status === 'failed'}
        <ErrorState message={`Agent usage did not load. ${store.loadProblem}`} {onRetry} />
      {:else if view === null}
        <LoadingState label="Loading agent usage" rows={8} />
      {:else}
        <p class="summary" aria-live="polite">{summaryLine(view.usage.total)}</p>
        {#if store.status === 'failed'}
          <ErrorState message={`Agent usage did not load. ${store.loadProblem}`} {onRetry} />
        {:else if view.usage.rows.length === 0}
          <EmptyState title="No use in these days" line="Change the days or the filters." />
        {:else}
          <div class="numbers">
            <Table {columns} rows={view.usage.rows} rowKey={usageRowKey} caption="Use of the AI agent programs" density="dense" />
          </div>
        {/if}
        <UsageNotes answer={view.usage} />
      {/if}
    </div>
  {/if}
</section>

<style>
  .usage {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-inline-size: 0;
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-block-size: calc(var(--control-sm) * 14);
  }

  .summary {
    margin: 0;
    font-size: var(--text-lg);
    line-height: var(--leading-tight);
  }

  /* Counts are figures, so they use the monospace of the tokens and line up on the right. */
  .numbers {
    overflow-x: auto;
  }

  .numbers :global(td.number) {
    font-family: var(--font-mono);
  }

  .action {
    color: var(--accent);
  }
</style>
