<script lang="ts">
  import { untrack } from 'svelte';
  import type { CardDevice } from '$lib/devices/api';
  import LogsPanel from '$lib/events/panel/LogsPanel.svelte';
  import { COLLAPSED } from '$lib/events/resize/resize';
  import { phoneLayout } from '$lib/nav/phone/layout.svelte';
  import '$lib/events/resize/geometry.css';
  import { hubIsHosted } from '$lib/hosted/mark';
  import { useHub } from '$lib/hub/context';
  import { capabilities } from '$lib/platform/platform';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import DeviceGrid from '../card/DeviceGrid.svelte';
  import DeviceTable from '../card/DeviceTable.svelte';
  import PhoneHead from '../phone/PhoneHead.svelte';
  import PhoneRows from '../phone/PhoneRows.svelte';
  import type { ListQuery } from '../query';
  import Toolbar from '../toolbar/Toolbar.svelte';
  import { Sites } from '../toolbar/sites.svelte';
  import { Tags } from '../toolbar/tags.svelte';
  import { isFiltered, withoutFilters } from '../toolbar/toolbar';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import { Listing } from './listing.svelte';
  import { browserClock, type LoadingClock } from '$lib/motion/loading/loading';
  import LiveNotice from '$lib/hub/notice/LiveNotice.svelte';
  import { type CardScreenSharing, type ScreenSharingCalls } from '$lib/devices/screen-sharing/setup';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WakeApi } from '$lib/sessions/wake/api';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import Skeleton from './Skeleton.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import Tally from '../list/Tally.svelte';
  import { grouped, tallyOf, type Tally as Counts } from '../list/look';

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
  // A hosted hub in a browser: the application adds a computer, so the empty list says so (HMD-066).
  const offersApplication = !capabilities().keyStore && hubIsHosted(hub.address.origin);
  // The clock is fixed for the life of the screen.
  // svelte-ignore state_referenced_locally
  const listing = new Listing(hub, clock);
  const tags = new Tags(hub);
  const sites = new Sites(hub);
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

  /** Which face the results show; a change of face cross-fades, a change inside one does not. */
  const face = $derived(listing.skeleton ? 'skeleton' : list === null ? 'none' : phoneLayout() ? 'phone' : query.view);

  /** The counts are of every device: while a count filters the list, they keep the last unfiltered ones (D18). */
  const NO_COUNTS: Counts = { total: 0, parts: [] };
  let tally = $state<Counts | null>(null);
  $effect(() => {
    // The first list gives the first counts, also when it is filtered: no count is better than "0 devices".
    if (list !== null && (query.statuses.length === 0 || untrack(() => tally) === null)) tally = tallyOf(list.devices);
  });

  $effect(() => {
    void listing.load(query);
  });
  $effect(() => {
    listing.start();
    void tags.load();
    void sites.load();
    return () => {
      listing.stop();
    };
  });
</script>

<!-- The phone layout has its own head and rows (mockup 4): one add button, the counts, the search, and no menus. -->
{#if phoneLayout()}
  <PhoneHead tally={tally ?? NO_COUNTS} {query} {onQueryChange} {onAddDevice} />
{:else}
<div class="head" {@attach entrance()}>
  <PageHeader title="Devices">
    {#snippet actions()}<FormButton tone="primary" icon="carbon:add" onclick={onAddDevice}>Add device</FormButton>{/snippet}
  </PageHeader>
  <Tally {tally} waiting={listing.error === null} {query} {onQueryChange} />
  <Toolbar {query} tags={tags.names} tagsNote={tags.note} sites={sites.options} sitesNote={sites.note} {onQueryChange} />
</div>
{/if}

<LiveNotice message={listing.liveError?.message ?? listing.charges.error?.message ?? listing.memory.error?.message ?? null} />

{#if listing.error !== null && list === null}
  <!-- Nothing to show: the failure takes the place of the list, in the look of every other failed load (UI-030). -->
  <ErrorState message={`Devices did not load. ${listing.error.message}`} onRetry={() => void listing.retry()} />
{:else if listing.error !== null}
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

<!-- One layer for each face: the skeleton, the list, the grid or the phone rows. A new face cross-fades over the old one
     in the same place (5.7.3, Loading); data that changes in a face stays in its layer and moves by its rows. -->
<div class="results" aria-busy={listing.skeleton}>
  {#key face}
    <div class="layer" transition:crossFade>
      {#if listing.skeleton}
        <Skeleton view={query.view} grouped={grouped(query)} />
      {:else if list === null}
        <!-- Nothing has loaded yet, and the skeleton waits for its threshold. -->
      {:else if list.devices.length === 0}
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
            {#if offersApplication}
              <!-- HMD-066: a browser may be the first sign-in of an account, and a hosted hub gets its computers from the application. -->
              <p class="how">Install the Anima.ing application on a computer and sign in there. The computer then shows here.</p>
            {/if}
            <button type="button" tabindex="0" onclick={onAddDevice} {@attach press()}>Add device</button>
          {/if}
        </div>
      {:else if phoneLayout()}
        <PhoneRows devices={list.devices} beats={listing.beats} {query} {onConnect} />
      {:else if query.view === 'table'}
        <DeviceTable devices={list.devices} beats={listing.beats} {query} {onConnect} {onOpenWeb} {wake} screenSharing={sharing} charges={listing.charges} memory={listing.memory} />
      {:else}
        <DeviceGrid devices={list.devices} beats={listing.beats} {query} {onConnect} {onOpenWeb} {wake} screenSharing={sharing} charges={listing.charges} memory={listing.memory} />
      {/if}
      {#if !listing.skeleton && list?.hasNextPage === true}
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
  {/key}
</div>

<!-- In the phone layout the logs are a page of the bar, not a panel over the list (MOB-013). -->
{#if !phoneLayout()}
  <div class="clearance" aria-hidden="true"></div>
  <!-- The panel starts as its one line, with the newest event on it, so the list has the page (R-224). -->
  <LogsPanel devices={list?.devices ?? []} start={COLLAPSED} />
{/if}

<style>
  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .head > :global(:last-child) {
    margin-block-start: var(--space-3);
  }

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
    color: var(--text-secondary);
    font-size: var(--text-lg);
  }

  .empty .how {
    max-inline-size: calc(var(--space-16) * 6);
    font-size: var(--text-md);
    text-align: center;
  }

  .more {
    justify-content: center;
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
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
