<script lang="ts">
  import { asset } from '$app/paths';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { markOf, pictureOf } from './marks';
  import { alsoSetsSentence, countLine, extensionRows } from './switches';
  import { useExtensions } from './store.svelte';

  /** The rows as one tight list, or as tiles; the page keeps the choice (EXT-001). */
  const { view = 'list' }: { view?: 'list' | 'grid' } = $props();

  // The Extensions page draws the heading; these are its rows. A change is saved at once (EXT-001).
  const store = useExtensions();
  const rows = $derived(extensionRows(store.shown));
  /** The switch-on that waits for the operator's yes, with the sentence of EXT-011. */
  let confirming = $state<{ key: string; name: string; sentence: string } | null>(null);

  $effect(() => {
    void store.ensureLoaded();
  });

  /** False keeps the switch where it was until the operator says yes. */
  function change(on: boolean, key: string, label: string): boolean {
    const sentence = on ? alsoSetsSentence(key, store.settled) : null;
    if (sentence === null) {
      void store.set(key, on);
      return true;
    }
    confirming = { key, name: label, sentence };
    return false;
  }

  async function confirmed(): Promise<{ ok: true; value: undefined }> {
    const waiting = confirming;
    confirming = null;
    if (waiting !== null) void store.set(waiting.key, true);
    return { ok: true, value: undefined };
  }
</script>

{#if store.status === 'failed'}
  <ErrorState message={store.loadProblem} onRetry={() => void store.load()} />
{:else}
  <!-- One extension in each row or tile: its mark, its name, one line, and its switch or why it has none. The list
       and the grid hold the same parts in the same order; only the layout differs (operator, 2026-11-19). -->
  {#if rows.length > 0}<p class="count">{countLine(rows)}</p>{/if}
  <ul class="rows" class:grid={view === 'grid'} aria-busy={store.status === 'loading'}>
    {#each rows as row (row.key)}
      {@const picture = view === 'grid' ? pictureOf(row.key, row.kind !== 'unbuilt') : null}
      <li class="extension" class:unbuilt={row.kind === 'unbuilt'}>
        {#if picture === null}
          <span class="mark"><Icon name={markOf(row.key)} size={view === 'grid' ? 'lg' : 'md'} /></span>
        {:else}
          <!-- A picture of the page of the extension, made from sample data; the one of the appearance in use shows. -->
          <span class="mark picture">
            <img class="dark" src={asset(picture.dark)} alt="" loading="lazy" />
            <img class="light" src={asset(picture.light)} alt="" loading="lazy" />
          </span>
        {/if}
        {#if row.kind === 'switch' || row.kind === 'needed'}
          <label class="name" for="extension-{row.key}">{row.label}</label>
        {:else}
          <span class="name">{row.label}</span>
        {/if}
        <p class="hint" id="extension-{row.key}-hint" title={row.hint}>{row.hint}</p>
        <span class="control">
          {#if row.kind === 'switch' || row.kind === 'needed'}
            <Switch
              id="extension-{row.key}"
              checked={row.on}
              disabled={row.kind === 'needed'}
              describedby="extension-{row.key}-hint"
              onchange={(on) => change(on, row.key, row.label)}
            />
          {:else if row.kind === 'fixed'}
            <span class="state">On</span>
          {:else}
            <span class="tag">Coming soon</span>
          {/if}
        </span>
        <p class="refusal" role="alert">{store.refusal?.key === row.key ? store.refusal.message : ''}</p>
      </li>
    {/each}
  </ul>
{/if}

{#key confirming}
  <ConfirmModal
    open={confirming !== null}
    title={confirming === null ? '' : `Set ${confirming.name} on`}
    body={confirming?.sentence ?? ''}
    confirmLabel="Set on"
    onConfirm={confirmed}
    onCancel={() => {
      confirming = null;
    }}
  />
{/key}

<style>
  .count {
    margin: 0 0 var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* The list: one tight line for each extension, with the line of words cut at the end; its full text is the tooltip. */
  .extension {
    display: grid;
    grid-template-columns: var(--icon-md) calc(var(--space-16) * 2.5) minmax(0, 1fr) auto;
    grid-template-areas: 'mark name hint control' '. refusal refusal refusal';
    column-gap: var(--space-3);
    align-items: center;
    min-block-size: var(--control-md);
    padding: var(--space-1);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .mark {
    grid-area: mark;
    display: inline-flex;
    color: var(--text-tertiary);
  }

  .name {
    grid-area: name;
    overflow: hidden;
    color: var(--text);
    font-weight: var(--weight-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    grid-area: hint;
    margin: 0;
    overflow: hidden;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .control {
    grid-area: control;
    display: inline-flex;
    justify-content: end;
    min-inline-size: var(--space-8);
  }

  .refusal {
    grid-area: refusal;
    margin: 0;
    color: var(--status-down);
    font-size: var(--text-sm);
  }

  .refusal:empty {
    display: none;
  }

  .state {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .tag {
    display: inline-flex;
    align-items: center;
    block-size: var(--space-5);
    padding-inline: var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--status-maintenance-tint);
    color: var(--planned);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  /* The grid: the same parts as a tile, with the mark large on a plain field and the words in full. */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(var(--space-16) * 3.5), 1fr));
    gap: var(--space-3);
  }

  .grid .extension {
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas: 'mark mark' 'name control' 'hint hint' 'refusal refusal';
    align-content: start;
    row-gap: var(--space-2);
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .grid .mark {
    place-content: center;
    place-items: center;
    block-size: calc(var(--space-16) * 1.25);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .grid .hint {
    white-space: normal;
  }

  /* The edge of the picture is drawn over it, so a tile with a picture has the height of a tile with a mark. */
  .grid .picture {
    overflow: hidden;
    padding: 0;
    outline: var(--border-thin) solid var(--border-subtle);
    outline-offset: calc(var(--border-thin) * -1);
  }

  .picture img {
    inline-size: 100%;
    block-size: 100%;
    object-fit: cover;
    object-position: top left;
  }

  /* The picture of the appearance in use: dark unless the theme, or the system for the theme "system", is light. */
  .picture .light,
  :global(:root[data-theme='light']) .picture .dark {
    display: none;
  }

  :global(:root[data-theme='light']) .picture .light {
    display: block;
  }

  @media (prefers-color-scheme: light) {
    :global(:root[data-theme='system']) .picture .dark {
      display: none;
    }

    :global(:root[data-theme='system']) .picture .light {
      display: block;
    }
  }
</style>
