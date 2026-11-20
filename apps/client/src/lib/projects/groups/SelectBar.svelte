<script lang="ts">
  import type { SvelteSet } from 'svelte/reactivity';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import type { ProjectsStore } from '../store.svelte';
  import NameModal from './NameModal.svelte';

  interface Props {
    store: ProjectsStore;
    /** The IDs of the selected projects; the bar adds and clears them. */
    selected: SvelteSet<string>;
    /** The IDs of each project that the list shows with the present filter: "Select all" takes these (PRO-060). */
    shown: readonly string[];
    onDone: () => void;
  }

  const { store, selected, shown, onDone }: Props = $props();
  let naming = $state(false);
  const count = $derived(selected.size);
  const none = $derived(count === 0);
  const allShown = $derived(shown.length > 0 && shown.every((id) => selected.has(id)));

  function selectAll(): void {
    if (allShown) selected.clear();
    else for (const id of shown) selected.add(id);
  }

  async function put(groupId: string | null): Promise<void> {
    await store.setGroup([...selected], groupId);
    if (store.refusal === null) selected.clear();
  }

  async function create(name: string) {
    const result = await store.createGroup(name, [...selected]);
    if (result.ok) selected.clear();
    return result;
  }
</script>

<!-- Select mode (PRO-060, PRO-063): how many are selected, then what to do with them. Each action names the selected
     projects; none starts with no project selected. -->
<div class="bar" role="toolbar" aria-label="Selected projects">
  <span class="count" aria-live="polite">{count === 1 ? '1 selected' : `${String(count)} selected`}</span>
  <FormButton tone="ghost" size="sm" onclick={selectAll} aria-disabled={shown.length === 0}>{allShown ? 'Clear selection' : 'Select all'}</FormButton>
  <span class="actions">
    <FormButton size="sm" onclick={() => (naming = true)} aria-disabled={none}>New group</FormButton>
    {#if store.groups.length > 0}
      <label>
        <span class="hidden">Add to group</span>
        <Select
          value=""
          disabled={none}
          onchange={(event) => {
            const value = event.currentTarget.value;
            event.currentTarget.value = '';
            if (value !== '') void put(value);
          }}
        >
          <option value="" disabled>Add to group</option>
          {#each store.groups.toSorted((a, b) => a.name.localeCompare(b.name)) as group (group.id)}
            <option value={group.id}>{group.name}</option>
          {/each}
        </Select>
      </label>
    {/if}
    <FormButton size="sm" onclick={() => void put(null)} aria-disabled={none}>Remove from group</FormButton>
    <FormButton tone="primary" size="sm" onclick={onDone}>Done</FormButton>
  </span>
</div>
<NameModal open={naming} title="New group" saveLabel="Make group" onSave={create} onClose={() => (naming = false)} />

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .count {
    color: var(--text);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-inline-start: auto;
  }

  .hidden {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
