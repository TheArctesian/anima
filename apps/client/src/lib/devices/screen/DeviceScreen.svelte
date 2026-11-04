<script lang="ts">
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import DeviceGrid from '../card/DeviceGrid.svelte';
  import DeviceTable from '../card/DeviceTable.svelte';
  import type { ListQuery } from '../query';
  import Toolbar from '../toolbar/Toolbar.svelte';
  import { Tags } from '../toolbar/tags.svelte';
  import { isFiltered, withoutFilters } from '../toolbar/toolbar';
  import { Listing } from './listing.svelte';
  import { browserClock, type LoadingClock } from './loading';
  import Skeleton from './Skeleton.svelte';

  interface Props {
    /** The query in the URL. */
    query: ListQuery;
    onQueryChange: (query: ListQuery) => void;
    onAddDevice: () => void;
    onConnect: (deviceId: string, button: HTMLElement) => void;
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { query, onQueryChange, onAddDevice, onConnect, clock = browserClock }: Props = $props();
  const hub = useHub();
  // The clock is fixed for the life of the screen.
  // svelte-ignore state_referenced_locally
  const listing = new Listing(hub, clock);
  const tags = new Tags(hub);
  const list = $derived(listing.list);

  $effect(() => {
    void listing.load(query);
  });
  $effect(() => {
    listing.start();
    void tags.load();
    return () => {
      listing.stop();
    };
  });
</script>

<Toolbar {query} tags={tags.names} tagsNote={tags.note} {onQueryChange} {onAddDevice} />

{#if listing.liveError !== null}
  <p class="notice" role="status">Live updates stopped: {listing.liveError.message}</p>
{/if}

{#if listing.error !== null}
  <div class="notice" role="alert">
    <p>{listing.error.message}</p>
    <button
      type="button"
      tabindex="0"
      onclick={() => {
        void listing.retry();
      }}
      {@attach press()}>Retry</button
    >
  </div>
{/if}

<div class="results" aria-busy={listing.skeleton}>
  {#if listing.skeleton}
    <div class="layer" transition:crossFade><Skeleton view={query.view} /></div>
  {:else if list !== null}
    <div class="layer" transition:crossFade>
      {#if list.devices.length === 0}
        <div class="empty">
          {#if isFiltered(query)}
            <p>No devices match</p>
            <button
              type="button"
              tabindex="0"
              onclick={() => {
                onQueryChange(withoutFilters(query));
              }}
              {@attach press()}>Clear filters</button
            >
          {:else}
            <p>No devices yet</p>
            <button type="button" tabindex="0" onclick={onAddDevice} {@attach press()}>Add device</button>
          {/if}
        </div>
      {:else if query.view === 'table'}
        <DeviceTable devices={list.devices} beats={listing.beats} {onConnect} />
      {:else}
        <DeviceGrid devices={list.devices} beats={listing.beats} {onConnect} />
      {/if}
      {#if list.hasNextPage}
        <div class="more">
          {#if listing.moreError !== null}<p role="alert">{listing.moreError.message}</p>{/if}
          <button
            type="button"
            tabindex="0"
            disabled={listing.loadingMore}
            onclick={() => {
              void listing.loadMore();
            }}
            {@attach press()}>Load more</button
          >
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .results {
    min-inline-size: 0;
    display: grid;
    margin-block-start: var(--space-4);
  }

  .layer {
    grid-area: 1 / 1;
    min-inline-size: 0;
  }

  .notice,
  .empty,
  .more {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-block-start: var(--space-4);
    color: var(--text);
  }

  .notice p,
  .empty p,
  .more p {
    margin: 0;
  }

  .empty {
    flex-direction: column;
    padding: var(--space-12);
    color: var(--text-muted);
    font-size: var(--text-lg);
  }

  .more {
    justify-content: center;
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not(:disabled),
  button:active:not(:disabled) {
    background: var(--border);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
