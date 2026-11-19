<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import ContainerTable from './ContainerTable.svelte';
  import ContainerDetailView from './detail/ContainerDetailView.svelte';
  import ContainerLogView from './log/ContainerLogView.svelte';
  import { LINE_CHOICES } from './rows';
  import StorageTables from './storage/StorageTables.svelte';
  import type { ContainersStore } from './store.svelte';

  /**
   * HOST-013: what Docker has on a device. The containers with the log or the details of the selected one, and the
   * images, the volumes and the networks (HOST-020). Stage 1 only reads: no control here changes the device.
   */
  const { store }: { store: ContainersStore } = $props();

  const VIEWS = [
    { value: 'containers', label: 'Containers' },
    { value: 'images', label: 'Images' },
    { value: 'volumes', label: 'Volumes' },
    { value: 'networks', label: 'Networks' },
  ] as const;
  type View = (typeof VIEWS)[number]['value'];

  let view = $state<View>('containers');
  const selected = $derived(store.containers.find((container) => container.id === store.selected) ?? null);
  const lineOptions = LINE_CHOICES.map((value) => ({ value, label: value }));

  /**
   * The lists of HOST-020 are read when the operator first opens one: the read of the sizes is slow. An arrow key only
   * moves the focus in the two choices here, so passing a choice on the way to a different one starts no read.
   */
  function show(next: View): void {
    view = next;
    if (next === 'networks') void store.storage.networks.open();
    else if (next !== 'containers') void store.storage.storage.open();
  }
</script>

<section class="containers" aria-label="Containers">
  <div class="views"><Segmented label="What to show" size="sm" arrowsChoose={false} options={[...VIEWS]} value={view} onchange={show} /></div>
  {#if view === 'containers'}
    <ContainerTable {store} />
    {#if selected !== null && store.shown === 'log'}
      <section class="below log" aria-label="Log of {selected.name}">
        <header class="bar">
          <h3>Log of {selected.name}</h3>
          <Segmented label="Lines" size="sm" arrowsChoose={false} options={lineOptions} value={store.lines} onchange={(lines) => void store.setLines(lines)} />
          <FormButton size="sm" icon="carbon:renew" busy={store.logStatus === 'loading'} onclick={() => void store.readLog()}>Refresh</FormButton>
          <FormButton size="sm" tone="ghost" icon="carbon:close" label="Close the log" onclick={() => { store.close(); }} />
        </header>
        {#if store.logStatus === 'failed'}
          <ErrorState message={store.logProblem} onRetry={() => void store.readLog()} />
        {:else if store.log !== null}
          <ContainerLogView log={store.log} container={selected.name} />
        {:else}
          <p class="reading" role="status">Reading the log…</p>
        {/if}
      </section>
    {:else if selected !== null}
      <section class="below detail" aria-label="Details of {selected.name}">
        <header class="bar">
          <h3>Details of {selected.name}</h3>
          <FormButton size="sm" icon="carbon:renew" busy={store.detailStatus === 'loading'} onclick={() => void store.readDetail()}>Refresh</FormButton>
          <FormButton size="sm" tone="ghost" icon="carbon:close" label="Close the details" onclick={() => { store.close(); }} />
        </header>
        {#if store.detailStatus === 'failed'}
          <ErrorState message={store.detailProblem} onRetry={() => void store.readDetail()} />
        {:else if store.detail !== null}
          <ContainerDetailView detail={store.detail} />
        {:else}
          <p class="reading" role="status">Reading the details…</p>
        {/if}
      </section>
    {/if}
  {:else}
    <StorageTables store={store.storage} kind={view} />
  {/if}
</section>

<style>
  .containers {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-inline-size: 0;
  }

  /* The choice of the list keeps its own width at the start of the row. */
  .views {
    display: flex;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  h3 {
    flex: 1;
    margin: 0;
    overflow: hidden;
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .below {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  .reading {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
