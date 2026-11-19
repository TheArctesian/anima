<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import { projectGroups } from './rows';
  import type { ContainersStore } from './store.svelte';

  /**
   * HOST-013: the containers of a device in groups by Compose project, each group with the one state of its project
   * (HOST-019), and, when the operator asked, the mark of a newer image (HOST-021).
   */
  const { store }: { store: ContainersStore } = $props();

  const groups = $derived(projectGroups(store.containers, store.updates));
  /** The count is of the rows that are shown: a container that is gone after a refresh is not counted. */
  const newer = $derived(groups.flatMap((group) => group.rows).filter((row) => row.update?.newer === true).length);
  const pressed = (id: string, shown: 'log' | 'detail') => id === store.selected && store.shown === shown;
</script>

{#if store.status === 'failed'}
  <!-- The hub's sentence says what to do: no SSH credential, no Docker, the device does not answer. -->
  <ErrorState message={store.problem} onRetry={() => void store.load()} />
{:else if store.status === 'ready' && groups.length === 0}
  <EmptyState title="No containers" line="Docker on this device has no container, running or stopped." />
{:else}
  <header class="bar">
    <p class="count" aria-live="polite">
      {store.containers.length} {store.containers.length === 1 ? 'container' : 'containers'}{#if store.updatesStatus === 'ready'}, {newer} with a newer image{/if}
    </p>
    <FormButton size="sm" tone="ghost" icon="carbon:update-now" busy={store.updatesStatus === 'loading'} onclick={() => void store.checkUpdates()}>Look for newer images</FormButton>
    <FormButton size="sm" icon="carbon:renew" busy={store.status === 'loading'} onclick={() => void store.load()}>Refresh</FormButton>
  </header>
  {#if store.updatesStatus === 'failed'}
    <p class="problem" role="alert">{store.updatesProblem}</p>
  {/if}
  <table>
    <caption>The containers of this device, by Compose project</caption>
    <thead>
      <tr><th scope="col">Name</th><th scope="col">State</th><th scope="col">Status</th><th scope="col">Image</th><th scope="col">Ports</th><th scope="col"><span class="hidden">Details and log</span></th></tr>
    </thead>
    {#each groups as group (group.project)}
      <tbody>
        <tr class="group">
          <th scope="rowgroup" colspan="6">
            {group.project}{#if group.state !== ''}<span class="state {group.tone}"><span class="dot" aria-hidden="true"></span>{group.state}</span>{/if}<span class="summary">{group.summary}</span>
          </th>
        </tr>
        {#each group.rows as row (row.id)}
          <tr class:selected={row.id === store.selected}>
            <th scope="row" class="text" title={row.name}>{row.name}<span class="id">{row.shortId}</span></th>
            <td><span class="state {row.tone}"><span class="dot" aria-hidden="true"></span>{row.state}</span></td>
            <td class="text" title={row.status}>{row.status}</td>
            <td title={row.image}>
              <span class="image"><span class="text mono">{row.image}</span>{#if row.update !== null}<span class="mark" class:newer={row.update.newer}>{row.update.words}</span>{/if}</span>
            </td>
            <td class="text mono" title={row.ports}>{row.ports}</td>
            <td class="action">
              <FormButton size="sm" tone="ghost" aria-pressed={pressed(row.id, 'detail')} aria-label="Details of {row.name}" onclick={() => void store.inspect(row.id)}>Details</FormButton>
              <FormButton size="sm" tone="ghost" aria-pressed={pressed(row.id, 'log')} aria-label="Log of {row.name}" onclick={() => void store.select(row.id)}>Log</FormButton>
            </td>
          </tr>
        {/each}
      </tbody>
    {/each}
  </table>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .count {
    flex: 1;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .problem {
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

  caption,
  .hidden {
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

  thead th:nth-child(1) {
    inline-size: 21%;
  }

  thead th:nth-child(2) {
    inline-size: 13%;
  }

  thead th:nth-child(3) {
    inline-size: 16%;
  }

  thead th:nth-child(5) {
    inline-size: 12%;
  }

  thead th:nth-child(6) {
    inline-size: calc(var(--control-lg) * 3 + var(--space-6));
  }

  .group th {
    block-size: var(--control-sm);
    background: var(--surface-sunken);
    color: var(--text-secondary);
    font-weight: var(--weight-medium);
  }

  .summary {
    margin-inline-start: var(--space-3);
    font-weight: var(--weight-regular);
  }

  .group .state {
    margin-inline-start: var(--space-3);
    font-weight: var(--weight-regular);
  }

  tbody tr:not(.group):hover,
  .selected {
    background: var(--surface-hover);
  }

  /* A long text is cut with an ellipsis; the full text is the cell's title (UI-027). */
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mono,
  .id {
    font-family: var(--font-mono);
  }

  .id {
    margin-inline-start: var(--space-2);
    color: var(--text-secondary);
  }

  .action {
    text-align: end;
    white-space: nowrap;
  }

  /* The image is cut with an ellipsis; the mark of a newer image keeps its full words. */
  .image {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  .image .text {
    min-inline-size: 0;
  }

  /* HOST-021: the mark is a word, with a border that repeats it (MAT-005). */
  .mark {
    flex: none;
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--text-sm);
  }

  .mark.newer {
    border-color: var(--status-warning);
    color: var(--text);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    white-space: nowrap;
  }

  /* The dot repeats the word beside it; colour is never the only sign (MAT-005). */
  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-unknown);
  }

  .up .dot {
    background: var(--status-up);
  }

  .down .dot {
    background: var(--status-down);
  }

  .warning .dot {
    background: var(--status-warning);
  }

</style>
