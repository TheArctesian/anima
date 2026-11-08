<script lang="ts">
  import type { CardDevice } from '$lib/devices/api';
  import LogsPanel from '$lib/events/panel/LogsPanel.svelte';
  import '$lib/events/resize/geometry.css';
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
  import { browserClock, type LoadingClock } from '$lib/motion/loading/loading';
  import LiveNotice from '$lib/hub/notice/LiveNotice.svelte';
  import { type CardScreenSharing, type ScreenSharingCalls } from '$lib/devices/screen-sharing/setup';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WakeApi } from '$lib/sessions/wake/api';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import Skeleton from './Skeleton.svelte';

  interface Props {
    /** The query in the URL. */
    query: ListQuery;
    onQueryChange: (query: ListQuery) => void;
    onAddDevice: () => void;
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
    /** The Web button of a card or row (WEB-003); hidden while no route gives it. */
    onOpenWeb?: (device: CardDevice, webInterface: WebInterface, button: HTMLElement) => void;
    /** The Wake button's call on a card (RA-031); hidden while no route gives it. */
    wake?: WakeApi;
    /** The calls of the Screen sharing button on a card or row (R-079); hidden while no route gives them. */
    screenSharing?: ScreenSharingCalls;
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { query, onQueryChange, onAddDevice, onConnect, onOpenWeb, wake, screenSharing, clock = browserClock }: Props = $props();
  const hub = useHub();
  // The clock is fixed for the life of the screen.
  // svelte-ignore state_referenced_locally
  const listing = new Listing(hub, clock);
  const tags = new Tags(hub);
  const list = $derived(listing.list);
  /** The session opens from the button, which is still shown; then the list shows VNC on (R-079). */
  const sharing = $derived<CardScreenSharing | undefined>(
    screenSharing === undefined
      ? undefined
      : {
          calls: screenSharing,
          onReady: (ready, button) => {
            const shown = list?.devices.find((device) => device.id === ready.id);
            if (shown !== undefined) onConnect(shown, 'VNC', button);
            listing.enableVnc(ready.id);
          },
        },
  );

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

<LiveNotice message={listing.liveError?.message ?? null} />

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
        <DeviceTable devices={list.devices} beats={listing.beats} {onConnect} {onOpenWeb} screenSharing={sharing} />
      {:else}
        <DeviceGrid devices={list.devices} beats={listing.beats} {onConnect} {onOpenWeb} {wake} screenSharing={sharing} />
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

<div class="clearance" aria-hidden="true"></div>
<LogsPanel devices={list?.devices ?? []} />

<style>
  /*
   * Room to scroll the last card clear of the logs panel at every stop: the 66 % stop plus the header. It is a constant, never
   * the live height, so a drag or a stop change reflows nothing. The 66vh is TWO_THIRDS_SHARE in lib/events/resize/resize.ts
   * (THIRD_SHARE is the 33 % stop); change them together.
   */
  .clearance {
    block-size: calc(var(--logs-two-thirds) + var(--logs-header-size));
  }

  /* A new order must not drag the scroll position after the card the browser anchored to. */
  .results {
    overflow-anchor: none;
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

  button:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  button:active:not(:disabled) {
    transition-duration: var(--duration-press);
    background: var(--accent-tint);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
