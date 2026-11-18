<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import ContainerLogView from './ContainerLogView.svelte';
  import { LINE_CHOICES, projectGroups } from './rows';
  import type { ContainersStore } from './store.svelte';

  /** HOST-013: the containers of a device in groups by Compose project, and the log of the selected one. */
  const { store }: { store: ContainersStore } = $props();

  const groups = $derived(projectGroups(store.containers));
  const selected = $derived(store.containers.find((container) => container.id === store.selected) ?? null);
  const lineOptions = LINE_CHOICES.map((value) => ({ value, label: value }));
</script>

<section class="containers" aria-label="Containers">
  {#if store.status === 'failed'}
    <!-- The hub's sentence says what to do: no SSH credential, no Docker, the device does not answer. -->
    <ErrorState message={store.problem} onRetry={() => void store.load()} />
  {:else if store.status === 'ready' && groups.length === 0}
    <EmptyState title="No containers" line="Docker on this device has no container, running or stopped." />
  {:else}
    <header class="bar">
      <p class="count" aria-live="polite">{store.containers.length} {store.containers.length === 1 ? 'container' : 'containers'}</p>
      <FormButton size="sm" icon="carbon:renew" busy={store.status === 'loading'} onclick={() => void store.load()}>Refresh</FormButton>
    </header>
    <table>
      <caption>The containers of this device, by Compose project</caption>
      <thead>
        <tr><th scope="col">Name</th><th scope="col">State</th><th scope="col">Status</th><th scope="col">Image</th><th scope="col">Ports</th><th scope="col"><span class="hidden">Log</span></th></tr>
      </thead>
      {#each groups as group (group.project)}
        <tbody>
          <tr class="group"><th scope="rowgroup" colspan="6">{group.project}<span class="summary">{group.summary}</span></th></tr>
          {#each group.rows as row (row.id)}
            <tr class:selected={row.id === store.selected}>
              <th scope="row" class="text" title={row.name}>{row.name}<span class="id">{row.shortId}</span></th>
              <td><span class="state {row.tone}"><span class="dot" aria-hidden="true"></span>{row.state}</span></td>
              <td class="text" title={row.status}>{row.status}</td>
              <td class="text mono" title={row.image}>{row.image}</td>
              <td class="text mono" title={row.ports}>{row.ports}</td>
              <td class="action">
                <FormButton size="sm" tone="ghost" aria-pressed={row.id === store.selected} aria-label="Log of {row.name}" onclick={() => void store.select(row.id)}>Log</FormButton>
              </td>
            </tr>
          {/each}
        </tbody>
      {/each}
    </table>
  {/if}

  {#if selected !== null}
    <section class="log" aria-label="Log of {selected.name}">
      <header class="bar">
        <h3>Log of {selected.name}</h3>
        <Segmented label="Lines" size="sm" options={lineOptions} value={store.lines} onchange={(lines) => void store.setLines(lines)} />
        <FormButton size="sm" icon="carbon:renew" busy={store.logStatus === 'loading'} onclick={() => void store.readLog()}>Refresh</FormButton>
        <FormButton size="sm" tone="ghost" icon="carbon:close" label="Close the log" onclick={() => { store.close(); }} />
      </header>
      {#if store.logStatus === 'failed'}
        <ErrorState message={store.logProblem} onRetry={() => void store.readLog()} />
      {:else if store.log !== null}
        <ContainerLogView log={store.log} />
      {:else}
        <p class="reading" role="status">Reading the log…</p>
      {/if}
    </section>
  {/if}
</section>

<style>
  .containers {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-inline-size: 0;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .count,
  h3 {
    flex: 1;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
  }

  h3 {
    overflow: hidden;
    color: var(--text);
    font-weight: var(--weight-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
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
    inline-size: 24%;
  }

  thead th:nth-child(2) {
    inline-size: 16%;
  }

  thead th:nth-child(3) {
    inline-size: 20%;
  }

  thead th:nth-child(6) {
    inline-size: calc(var(--control-lg) + var(--space-6));
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

  .log {
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
