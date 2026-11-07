<script lang="ts">
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import type { SocketConstructor } from '../tunnel/state';
  import { filesApi, type FilesApi } from './api';
  import type { EntryKind } from './link/codec';
  import { FilesPanelState } from './panel.svelte';
  import { fraction, sizeText, timeText } from './paths';
  import type { BlobSaver } from './save';

  interface Props {
    /** The live SSH session this panel sits beside (K4 §9). */
    sessionId: string;
    /** The files calls; defaults to the hub of the page (`useHub`). */
    api?: FilesApi;
    /** `hub.address.origin`; defaults to the hub of the page. */
    origin?: string;
    /** The WebSocket constructor; defaults to the global one (CQ-021). */
    WebSocket?: SocketConstructor;
    /** Hands a finished download to the operator; defaults to an `<a download>` (CQ-021). */
    save?: BlobSaver;
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { sessionId, api, origin, WebSocket, save, clock }: Props = $props();

  /** The hub of the page stands in for whatever the parent did not pass. */
  function hubOf() {
    const hub = useHub();
    return { api: api ?? filesApi(hub), origin: origin ?? hub.address.origin };
  }

  function begin() {
    return new FilesPanelState({ ...(api === undefined || origin === undefined ? hubOf() : { api, origin }), sessionId, WebSocket, save });
  }

  const files = begin();
  const ICONS: Readonly<Record<EntryKind, IconName>> = { dir: 'nf-md-folder', file: 'nf-md-file', link: 'nf-md-link' };

  let picker: HTMLInputElement | undefined = $state();
  let dragging = $state(false);
  const idle = $derived(files.idle);
  // The clock is fixed for the life of the panel.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => files.listLoading, clock);
  /** Placeholder rows that fill the column's first screen. */
  const PLACES = Array.from({ length: 8 }, (_, index) => index);

  $effect(() => {
    void files.open();
    return () => {
      files.close();
    };
  });

  function upload(file: File | undefined) {
    if (file !== undefined && idle) void files.upload(file);
  }

  function picked() {
    upload(picker?.files?.[0]);
    if (picker !== undefined) picker.value = '';
  }

  function dropped(event: DragEvent) {
    event.preventDefault();
    dragging = false;
    upload(event.dataTransfer?.files[0]);
  }
</script>

<!-- A solid column beside the terminal, never over it (MAT-008). Dropping a file is a shortcut for the Upload button. -->
<section
  class="files"
  class:dragging
  aria-label="Files on the device"
  ondragover={(event) => {
    event.preventDefault();
    dragging = idle;
  }}
  ondragleave={() => (dragging = false)}
  ondrop={dropped}
>
  <div class="bar">
    <button type="button" class="control" aria-label="Up one folder" aria-disabled={!idle || files.listing === undefined} onclick={() => files.up()} {@attach press()}>
      <Icon name="nf-md-arrow_up" size="sm" />
    </button>
    <span class="where" title={files.listing?.path ?? ''}>{files.listing?.path ?? ''}</span>
    <button type="button" class="control" aria-label="Refresh the list" aria-disabled={!idle || files.listing === undefined} onclick={() => files.refresh()} {@attach press()}>
      <Icon name="nf-md-refresh" size="sm" />
    </button>
    <button type="button" class="control labelled" aria-label="Upload a file" aria-disabled={!idle || files.listing === undefined} onclick={() => idle && picker?.click()} {@attach press()}>
      <Icon name="nf-md-upload" size="sm" />
      <span>Upload</span>
    </button>
    <input bind:this={picker} class="picker" type="file" tabindex="-1" aria-hidden="true" onchange={picked} />
  </div>

  <!-- The placeholders lie over the rows in one cell and fade off them, so the swap moves nothing (MOT-007). -->
  <div class="stack">
    <ul class="entries" aria-label="Entries" aria-busy={files.listLoading}>
      <!-- Unkeyed: a device may list two entries with one name, and a repeated key would end the list. -->
      {#each files.entries as entry, at (at)}
        <li class="entry">
          <Icon name={ICONS[entry.kind]} size="sm" />
          {#if entry.kind === 'dir'}
            <button type="button" class="name open" title={entry.name} aria-disabled={!idle} onclick={() => files.into(entry)}>{entry.name}</button>
          {:else}
            <span class="name" title={entry.name}>{entry.name}</span>
          {/if}
          <span class="meta">{sizeText(entry.size)}</span>
          <span class="meta">{timeText(entry.modifiedAt)}</span>
          {#if entry.kind === 'dir'}
            <span class="slot"></span>
          {:else}
            <button type="button" class="control slot" aria-label="Download {entry.name}" aria-disabled={!idle} onclick={() => void files.download(entry)} {@attach press()}>
              <Icon name="nf-md-download" size="sm" />
            </button>
          {/if}
        </li>
      {/each}
    </ul>
    {#if skeleton.visible}
      <ul class="entries places" aria-hidden="true" transition:crossFade>
        {#each PLACES as place (place)}
          <li class="entry">
            <Icon name="nf-md-file" size="sm" />
            <span class="hole name-hole"></span>
            <span class="hole"></span>
            <span class="hole"></span>
            <span class="slot"></span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <!-- Both lines keep their height whether or not they have anything to say (MOT-007). -->
  <div class="transfer">
    <div class="track" class:quiet={files.transfer === undefined}>
      <div
        class="fill"
        role="progressbar"
        aria-label={files.transfer?.direction === 'upload' ? 'Upload progress' : 'Download progress'}
        aria-valuemin="0"
        aria-valuemax="1"
        aria-valuenow={fraction(files.transfer?.done ?? 0, files.transfer?.size)}
        style:inline-size="{fraction(files.transfer?.done ?? 0, files.transfer?.size) * 100}%"
      ></div>
    </div>
    <button type="button" class="control labelled" aria-disabled={files.transfer === undefined} onclick={() => files.cancel()} {@attach press()}>Cancel</button>
  </div>
  <p class="status" role="status" title={files.status}>{files.status}</p>
</section>

<style>
  .files {
    order: 1;
    box-sizing: border-box;
    display: flex;
    flex: none;
    flex-direction: column;
    inline-size: calc(var(--space-16) * 6);
    max-inline-size: 50%;
    min-block-size: 0;
    border-inline-start: var(--border-thin) solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .files.dragging {
    outline: var(--border-focus) dashed var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .bar,
  .transfer {
    box-sizing: border-box;
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-2);
    block-size: calc(var(--target-min-desktop) + 2 * var(--space-2));
    padding: 0 var(--space-3);
  }

  .bar {
    border-bottom: var(--border-thin) solid var(--border);
  }

  .where {
    flex: 1;
    min-inline-size: 0;
    overflow: hidden;
    font-family: var(--font-mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .picker {
    display: none;
  }

  /* One set of columns for every row, so the size and date columns line up and the name gets what is left. */
  .entries {
    display: grid;
    flex: 1;
    grid-template-columns: auto minmax(0, 1fr) auto auto var(--target-min-desktop);
    grid-auto-rows: calc(var(--target-min-desktop) + var(--space-2));
    align-content: start;
    column-gap: var(--space-2);
    min-block-size: 0;
    margin: 0;
    padding: 0 var(--space-3);
    overflow: auto;
    list-style: none;
  }

  /* The placeholders and the rows share one cell; the cell takes the column's height. */
  .stack {
    display: grid;
    flex: 1;
    grid-template-columns: minmax(0, 1fr);
    min-block-size: 0;
  }

  .stack > .entries {
    grid-area: 1 / 1;
  }

  .places {
    overflow: hidden;
    background: var(--surface);
  }

  /* A placeholder cell is an empty box; the row's height comes from the grid, so nothing here moves the rows (MOT-007). */
  .hole {
    min-inline-size: var(--space-8);
    block-size: var(--space-3);
    background: var(--surface-raised);
  }

  .name-hole {
    inline-size: 100%;
    max-inline-size: calc(var(--space-16) * 2);
  }

  .entry {
    display: grid;
    grid-template-columns: subgrid;
    grid-column: 1 / -1;
    align-items: center;
    min-inline-size: 0;
  }

  .name {
    overflow: hidden;
    font-family: var(--font-mono);
    text-align: start;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .open {
    min-block-size: var(--target-min-desktop);
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: inherit;
  }

  .open[aria-disabled="true"] {
    color: var(--text-muted);
  }

  .meta {
    color: var(--text-muted);
    white-space: nowrap;
  }

  .control {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-inline-size: var(--target-min-desktop);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  .control[aria-disabled="true"] {
    color: var(--text-muted);
  }

  .control:focus-visible,
  .open:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .slot {
    min-inline-size: var(--target-min-desktop);
  }

  /* The bar's value moves with the bytes and nothing else: no transition, no easing (MOT-009). */
  .track {
    flex: 1;
    overflow: hidden;
    block-size: var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
  }

  .track.quiet {
    visibility: hidden;
  }

  .fill {
    block-size: 100%;
    background: var(--accent);
  }

  /* Two lines are reserved, so the longest fixed sentence fits and the list above never moves (MOT-007). */
  .status {
    box-sizing: content-box;
    display: -webkit-box;
    flex: none;
    block-size: calc(2 * var(--text-sm) * var(--leading-normal));
    margin: 0;
    padding: var(--space-1) var(--space-3);
    overflow: hidden;
    color: var(--text-muted);
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
</style>
