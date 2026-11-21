<script lang="ts">
  import { tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import { SHELL_REASON, TIER_LABELS, rowLine, unreviewed, type TierChoice } from './entry';
  import type { CatalogueStore } from './store.svelte';

  /**
   * Task H39: the entries of the catalogue in groups by category, with a search and a choice of the tier, so that the
   * list holds about 200 entries. Only the list scrolls; each group head stays at the top while its rows pass. The arrow
   * keys move the selection from row to row.
   */
  const { store }: { store: CatalogueStore } = $props();
  const TIERS: readonly TierChoice[] = ['ALL', 'TESTED', 'GENERATED'];
  const tierOptions = $derived(TIERS.map((value) => ({ value, label: TIER_LABELS[value], count: store.counts[value] })));
  const groups = $derived(store.groups);
  const shown = $derived(groups.reduce((sum, group) => sum + group.entries.length, 0));
  let list = $state<HTMLElement>();

  async function arrow(event: KeyboardEvent): Promise<void> {
    const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    store.move(step);
    await tick();
    list?.querySelector<HTMLButtonElement>('button[aria-pressed="true"]')?.focus();
  }
</script>

<section class="list" aria-label="Entries">
  <div class="tools">
    <div class="search"><Input type="search" icon="carbon:search" aria-label="Search the catalogue" placeholder="Search by name or tag" value={store.search} oninput={(event) => { store.filter(event.currentTarget.value, store.tier); }} /></div>
    <Segmented label="Tier" size="sm" options={tierOptions} value={store.tier} onchange={(tier) => { store.filter(store.search, tier); }} />
  </div>
  <p class="shown" role="status">{shown} {shown === 1 ? 'entry' : 'entries'}</p>
  {#if shown === 0}
    <EmptyState title="No entry matches the search" line="Clear the search, or choose a different tier.">
      {#snippet action()}
        <FormButton size="sm" onclick={() => { store.filter('', 'ALL'); }}>Clear the search</FormButton>
      {/snippet}
    </EmptyState>
  {:else}
    <ul class="groups" bind:this={list}>
      {#each groups as group (group.category)}
        <li>
          <h4 class="head">{group.category}<span class="count">{group.entries.length}</span></h4>
          <ul class="entries">
            {#each group.entries as entry (entry.id)}
              <li>
                <button type="button" class="entry" aria-pressed={entry.id === store.selected} onkeydown={(event) => void arrow(event)} onclick={() => { store.select(entry.id === store.selected ? null : entry.id); }}>
                  <span class="lines">
                    <span class="name">{entry.name}{#if entry.class === 'SHELL'}<span class="mark">Shell</span>{/if}{#if unreviewed(entry)}<span class="mark">Not reviewed</span>{/if}</span>
                    <span class="line">{rowLine(entry)}</span>
                    {#if entry.class === 'SHELL'}<span class="line quiet">{SHELL_REASON}</span>{/if}
                  </span>
                  <span class="mark">{TIER_LABELS[entry.tier]}</span>
                </button>
              </li>
            {/each}
          </ul>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-block-size: 0;
    min-inline-size: 0;
    padding-block-start: var(--space-3);
    overflow: hidden;
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding-inline: var(--space-4);
  }

  .search {
    flex: 1;
    min-inline-size: 12rem;
  }

  .shown {
    margin: 0;
    padding-inline: var(--space-4);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  .groups,
  .entries {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .groups {
    flex: 1;
    min-block-size: 0;
    overflow: auto;
  }

  .head {
    position: sticky;
    inset-block-start: 0;
    z-index: var(--layer-sticky);
    display: flex;
    gap: var(--space-3);
    margin: 0;
    padding: var(--space-1) var(--space-4);
    background: var(--surface-base);
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .count {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .entry {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-3);
    inline-size: 100%;
    min-block-size: var(--control-lg);
    padding: var(--space-2) var(--space-4);
    border: 0;
    border-block-start: var(--border-thin) solid var(--border-subtle);
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

  .lines {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }

  .name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: var(--weight-medium);
  }

  .line {
    overflow: hidden;
    color: var(--text-secondary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quiet {
    color: var(--text-tertiary);
  }

  /* A neutral mark, as the mark of an image in the list of containers: the tier and the class are not a status. */
  .mark {
    flex: none;
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-weight: var(--weight-regular);
    white-space: nowrap;
  }
</style>
