<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import { enumWords, machineTest, markWords } from './entry';
  import type { CatalogueStore } from './store.svelte';

  /**
   * Task H38: the catalogue of applications that ships with Anima, on the Containers tab of the device that an install
   * would go to. The install button is present and refused with the hub's fixed reason until the install exists (task H6
   * of the hosted applications plan). Nothing here reaches the device.
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
    <ul class="entries">
      {#each store.entries as entry (entry.id)}
        <li>
          <button type="button" class="entry" aria-pressed={entry.id === store.selected} onclick={() => { store.select(entry.id === store.selected ? null : entry.id); }}>
            <span class="name">{entry.name}</span>
            <span class="version">{entry.version}</span>
            <span class="kind">{enumWords(entry.kind)}</span>
            <span class="tested">{machineTest(entry)}</span>
          </button>
        </li>
      {/each}
    </ul>
    {#if shown !== null}
      <section class="below" aria-label="About {shown.name}">
        <header class="bar">
          <h3>{shown.name}</h3>
          <FormButton size="sm" tone="primary" aria-disabled={shown.install.refusal !== null} aria-describedby="install-reason">Install on this device</FormButton>
          <FormButton size="sm" tone="ghost" icon="carbon:close" label="Close the description" onclick={() => { store.select(null); }} />
        </header>
        <p class="reason" id="install-reason" role="note">{shown.install.reason ?? 'The hub can install this entry.'}</p>
        {#if shown.install.needsOpenToNetwork}
          <p class="reason">It needs the setting Open to the network at the machine.</p>
        {/if}
        <p class="description">{shown.description}</p>
        <dl class="facts">
          <dt>Kind</dt><dd>{enumWords(shown.kind)}</dd>
          <dt>Version</dt><dd>{shown.version}</dd>
          <dt>Author</dt><dd>{shown.author}</dd>
          <dt>Tags</dt><dd>{shown.tags.join(', ')}</dd>
          <dt>Machine test</dt><dd>{machineTest(shown)}</dd>
          <dt>Images pinned</dt><dd>{shown.pinned}</dd>
          <dt>Marks</dt><dd>{markWords(shown)}</dd>
          <dt>Variables</dt><dd class="mono">{shown.environmentVariables.join(', ')}</dd>
          <dt>Documentation</dt>
          <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an https:// address of the recipe, outside the application -->
          <dd><a href={shown.documentationUrl} target="_blank" rel="noopener noreferrer">{shown.documentationUrl}</a></dd>
        </dl>
        <h4>Compose file</h4>
        <pre class="file">{shown.composeFile}</pre>
      </section>
    {/if}
  {/if}
</section>

<style>
  .catalogue {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-inline-size: 0;
  }

  .entries {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .entry {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr) minmax(0, 2fr);
    gap: var(--space-3);
    inline-size: 100%;
    padding: var(--space-2) var(--space-3);
    border: 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
    background: none;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: start;
    cursor: pointer;
  }

  .entry:hover {
    background: var(--surface-hover);
  }

  .entry[aria-pressed='true'] {
    background: var(--surface-selected);
  }

  .name {
    font-weight: var(--weight-medium);
  }

  .version,
  .kind,
  .tested {
    color: var(--text-secondary);
  }

  /* The description is one raised card under the list, as the log of a container (R-224). */
  .below {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  h3 {
    flex: 1;
    margin: 0;
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  h4 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .reason {
    color: var(--status-warning-text);
  }

  .description {
    color: var(--text);
  }

  .facts {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--space-1) var(--space-4);
    margin: 0;
    font-size: var(--text-sm);
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
    color: var(--text);
  }

  .mono,
  .file {
    font-family: var(--font-mono);
  }

  .file {
    margin: 0;
    padding: var(--space-3);
    overflow: auto;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text);
    font-size: var(--text-sm);
  }
</style>
