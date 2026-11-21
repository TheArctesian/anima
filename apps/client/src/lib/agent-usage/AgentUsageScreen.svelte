<script lang="ts">
  import { resolve } from '$app/paths';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import { batteryBar } from '$lib/metrics/battery/battery';
  import type { LiveCharges } from '$lib/metrics/battery/charges.svelte';
  import { utcDay, type UsageQuery } from './query';
  import { sourceName, stateWords } from './report/words';
  import type { UsageStore } from './store.svelte';
  import UsageFilters from './UsageFilters.svelte';
  import RepositoryList from './view/RepositoryList.svelte';
  import { dayBars, repositoryLines, type Picked } from './view/shape';
  import ShareTable, { type ShareRow } from './view/ShareTable.svelte';
  import TotalsCard from './view/TotalsCard.svelte';

  const REPORTS = 'The daemon of each device reports the use of the AI agent programs.';
  /** The daemon is installed for one device, in its settings; the Devices page is where a device is opened. */
  const WHERE_TO_INSTALL = 'To install it, open a device, then Settings, then Daemon.';

  interface Props {
    store: UsageStore;
    query: UsageQuery;
    onQueryChange: (query: UsageQuery) => void;
    onRetry: () => void;
    /**
     * On a hub that Anima.ing hosts: where the daemon of this computer stands, in one sentence (HMD-053). The
     * application installs it with no control, so no action is offered. Not set on any other hub.
     */
    hostedLine?: string;
    /** The time of now; a test gives its own. */
    now?: () => Date;
    /** The latest charge of each battery of each device (MET-032). */
    charges?: Pick<LiveCharges, 'of'>;
  }

  const { store, query, onQueryChange, onRetry, hostedLine, now = () => new Date(), charges }: Props = $props();
  const view = $derived(store.view);
  /** No daemon sent counts, whatever the choices: the whole page is the empty state. */
  const noDaemon = $derived(view !== null && view.everyDevice.length === 0);

  /** The program and model that limit the chart: a choice of this page only, since the hub's filter has no model. */
  let picked = $state<Picked | null>(null);
  const keyOf = (row: Picked): string => `${row.source ?? ''}\n${row.model ?? ''}`;
  const pickedKey = $derived(picked === null ? null : keyOf(picked));
  /** A picked row that the new answer does not have is no choice any more. */
  const shown = $derived(view !== null && picked !== null && view.models.some((row) => keyOf(row) === pickedKey) ? picked : null);

  const modelRows = $derived<ShareRow[]>(
    (view?.models ?? []).map((row) => ({ key: keyOf(row), name: sourceName(row.source), detail: row.model ?? 'No model', sums: row })),
  );
  /** Each device that reports, with its counts; one whose daemon sends none has its state and no number (AIS-024). */
  const deviceRows = $derived<ShareRow[]>(
    (view?.states ?? []).map(({ device, state }) => ({
      key: device.id,
      name: device.name,
      state: stateWords(state),
      battery: batteryBar(device.batteries, charges?.of(device.id)),
      sums: view?.devices.find((row) => row.device.id === device.id) ?? null,
    })),
  );
  const only = $derived(shown === null ? '' : `${sourceName(shown.source)} ${shown.model ?? 'No model'}`);

  function pick(key: string | null): void {
    const row = key === null ? undefined : view?.models.find((each) => keyOf(each) === key);
    picked = row === undefined ? null : { source: row.source, model: row.model };
  }
</script>

<!-- The use of the AI agent programs (AIS-020): the repositories at the side; the totals with one bar for each day; the
     use by program and model and by device (AIS-021, AIS-024). Counts only: the hub holds no single session and no text. -->
<section class="usage">
  <PageHeader title="Agent usage">
    {#snippet actions()}
      {#if !noDaemon}<UsageFilters {query} devices={view?.everyDevice ?? []} onChange={onQueryChange} />{/if}
    {/snippet}
  </PageHeader>
  {#if noDaemon}
    {#if hostedLine !== undefined}
      <EmptyState title="No use counts yet" line={`${REPORTS} ${hostedLine}`} />
    {:else}
      <EmptyState title="No use counts yet" line={`${REPORTS} ${WHERE_TO_INSTALL}`}>
        {#snippet action()}
          <a class="action" href={resolve('/devices')}>Open Devices</a>
        {/snippet}
      </EmptyState>
    {/if}
  {:else}
    <!-- The result keeps the height of a typical page while it loads, so nothing below it moves (MOT-007). -->
    <div class="result" aria-busy={store.status === 'loading'}>
      {#if view === null && store.status === 'failed'}
        <ErrorState message={`Agent usage did not load. ${store.loadProblem}`} {onRetry} />
      {:else if view === null}
        <LoadingState label="Loading agent usage" rows={8} />
      {:else}
        {#if store.status === 'failed'}
          <ErrorState message={`Agent usage did not load. ${store.loadProblem}`} {onRetry} />
        {/if}
        <div class="columns">
          <RepositoryList
            lines={repositoryLines(view)}
            everything={view.everything}
            days={query.days}
            chosen={query.repository}
            today={utcDay(now().getTime())}
            leftOut={view.repositoriesLeftOut}
            onChoose={(repository) => {
              onQueryChange({ ...query, repository });
            }}
          />
          <div class="main">
            <TotalsCard total={view.total} pricesAsOf={view.pricesAsOf} bars={dayBars(view, query.count, shown)} count={query.count} {only} />
            {#if view.total.sessions === 0}
              <EmptyState title="No use in these days" line="Change the days or the filters." />
            {:else}
              <ShareTable
                label="By program and model"
                note="select a row to see only its days above"
                head="Program and model"
                rows={modelRows}
                count={query.count}
                picked={shown === null ? null : pickedKey}
                onPick={pick}
              />
            {/if}
            <ShareTable
              label="By device"
              note="only devices whose daemon reports; counts only, no text of a session leaves a device"
              head="Device"
              rows={deviceRows}
              count={query.count}
            />
          </div>
        </div>
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

  .columns {
    display: grid;
    /* The repositories take the width of their longest name, from 224 px up to 352 px, so a name is cut only when it is
       longer than that. */
    grid-template-columns: fit-content(calc(var(--space-16) * 5.5)) minmax(0, 1fr);
    gap: var(--space-6);
    align-items: start;
  }

  .columns > :global(:first-child) {
    min-inline-size: calc(var(--space-16) * 3.5);
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    min-inline-size: 0;
  }

  .action {
    color: var(--accent);
  }

  /* In a narrow window the repositories stand over the totals. */
  @media (width <= 60rem) {
    .columns {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
