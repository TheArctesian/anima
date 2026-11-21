<script lang="ts">
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import CatalogueEntryPane from './CatalogueEntryPane.svelte';
  import CatalogueList from './CatalogueList.svelte';
  import type { CatalogueStore } from './store.svelte';

  /**
   * Tasks H38 and H39: the catalogue of applications that ships with Anima, on the Containers tab of the device that an
   * install would go to: the list on the left and the description of the selected entry on the right. Nothing here
   * reaches the device.
   */
  const { store }: { store: CatalogueStore } = $props();
  const shown = $derived(store.shown);
</script>

<section class="catalogue" aria-label="Catalogue">
  {#if store.status === 'failed'}
    <ErrorState message={store.problem} onRetry={() => void store.load()} />
  {:else if store.status !== 'ready'}
    <LoadingState label="Loading the catalogue" />
  {:else if store.entries.length === 0}
    <EmptyState title="No entry in the catalogue" line="The hub lists no recipe that passed its static test." />
  {:else}
    <p class="about">The applications that ship with Anima. The hub reads this list from its own program and connects to no machine.</p>
    <div class="split">
      <CatalogueList {store} />
      {#if shown !== null}
        <CatalogueEntryPane entry={shown} onclose={() => { store.select(null); }} />
      {/if}
    </div>
  {/if}
</section>

<style>
  .catalogue {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
  }

  .about {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  /* The list keeps its place when the description closes (Frame 3 of device-containers-v2). */
  .split {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-4);
    align-items: start;
  }

  /* Only the list and the description scroll, each in its own column. */
  .split > :global(*) {
    max-block-size: 70vh;
  }
</style>
