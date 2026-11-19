<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import { imageRows, networkRows, usageLine, volumeRows, type StorageKind } from './rows';
  import type { StorageStore } from './storage.svelte';

  /**
   * HOST-020: the images, the volumes or the networks of a device, read-only, with the size of each image and volume,
   * the containers that use it, and the space that a clean-up can free. Anima removes nothing in stage 1.
   */
  const { store, kind }: { store: StorageStore; kind: StorageKind } = $props();

  const read = $derived(kind === 'networks' ? store.networks : store.storage);
  const table = $derived(
    kind === 'images' ? imageRows(store.storage.value?.images ?? []) : kind === 'volumes' ? volumeRows(store.storage.value?.volumes ?? []) : networkRows(store.networks.value ?? []),
  );
  const usage = $derived(kind === 'networks' ? '' : usageLine(store.storage.value?.usage ?? [], kind));
</script>

{#if read.status === 'failed'}
  <ErrorState message={read.problem} onRetry={() => void read.load()} />
{:else if read.value === null}
  <p class="reading" role="status">Reading the {kind} of the device…</p>
{:else if table.rows.length === 0}
  <EmptyState title="No {kind}" line="Docker on this device has none." />
{:else}
  <header class="bar">
    <p class="count" aria-live="polite">{table.rows.length} {kind}{usage === '' ? '' : `. ${usage}`}</p>
    <FormButton size="sm" icon="carbon:renew" busy={read.status === 'loading'} onclick={() => void read.load()}>Refresh</FormButton>
  </header>
  <table>
    <caption>The {kind} of Docker on this device</caption>
    <thead>
      <tr>{#each table.columns as column (column)}<th scope="col">{column}</th>{/each}</tr>
    </thead>
    <tbody>
      {#each table.rows as row (row.key)}
        <tr>
          <th scope="row" class="text mono" title={row.cells[0]}>{row.cells[0]}</th>
          {#each row.cells.slice(1) as cell, index (index)}<td class="text" title={cell}>{cell}</td>{/each}
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .count,
  .reading {
    flex: 1;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  table {
    inline-size: 100%;
    table-layout: fixed;
    border-collapse: separate;
    border-spacing: 0;
    color: var(--text);
    font-size: var(--text-sm);
  }

  caption {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  th,
  td {
    box-sizing: border-box;
    block-size: var(--control-lg);
    padding: 0 var(--space-3);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    font-weight: var(--weight-regular);
    text-align: start;
    vertical-align: middle;
  }

  thead th {
    color: var(--text-secondary);
    font-weight: var(--weight-medium);
  }

  thead th:first-child {
    inline-size: 34%;
  }

  tbody tr:hover {
    background: var(--surface-hover);
  }

  /* A long text is cut with an ellipsis; the full text is the cell's title (UI-027). */
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
