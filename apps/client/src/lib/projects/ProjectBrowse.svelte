<script lang="ts">
  import Select from '$lib/forms/input/Select.svelte';
  import type { ProjectBrowse } from './api';
  import type { ProjectsStore } from './store.svelte';

  const { store }: { store: ProjectsStore } = $props();

  const ORDERS = {
    NAME: 'Grouped by status',
    LAST_COMMIT: 'Last commit',
    CREATED: 'Date created',
    COMMIT_COUNT: 'Commit count',
    STARS: 'Stars',
  } as const satisfies Record<NonNullable<ProjectBrowse['order']>, string>;

  const SIZES = [
    { label: 'Any size', min: null, max: null },
    { label: 'Under 1 MiB', min: null, max: 1023 },
    { label: '1 to 10 MiB', min: 1024, max: 10_239 },
    { label: '10 to 100 MiB', min: 10_240, max: 102_399 },
    { label: '100 MiB or more', min: 102_400, max: null },
  ] as const;

  function order(value: string): void {
    const selected = Object.keys(ORDERS).find((key): key is keyof typeof ORDERS => key === value);
    if (selected !== undefined) void store.setBrowse({ order: selected });
  }

  const size = $derived(SIZES.findIndex((range) => range.min === (store.browse.minSizeKib ?? null) && range.max === (store.browse.maxSizeKib ?? null)));

  /** The value of the group filter for "Ungrouped": no group ID looks like it (PRO-061). */
  const NO_GROUP = 'ungrouped';
  const group = $derived(store.browse.ungrouped === true ? NO_GROUP : (store.browse.group ?? ''));

  function chooseGroup(value: string): void {
    void store.setBrowse({ group: value === '' || value === NO_GROUP ? null : value, ungrouped: value === NO_GROUP ? true : null });
  }

  function chooseSize(value: string): void {
    const range = SIZES[Number(value)];
    if (range !== undefined) void store.setBrowse({ minSizeKib: range.min, maxSizeKib: range.max });
  }
</script>

<div class="browse">
  {#if store.groups.length > 0 || group !== ''}
    <label>
      Group
      <Select value={group} onchange={(event) => chooseGroup(event.currentTarget.value)}>
        <option value="">All groups</option>
        {#each store.groups.toSorted((a, b) => a.name.localeCompare(b.name)) as each (each.id)}
          <option value={each.id}>{each.name}</option>
        {/each}
        <option value={NO_GROUP}>Ungrouped</option>
      </Select>
    </label>
  {/if}
  <label>
    Sort
    <Select value={store.browse.order ?? 'NAME'} onchange={(event) => order(event.currentTarget.value)}>
      {#each Object.entries(ORDERS) as [value, label] (value)}
        <option {value}>{label}</option>
      {/each}
    </Select>
  </label>
  <label>
    Language
    <Select value={store.browse.language ?? ''} onchange={(event) => void store.setBrowse({ language: event.currentTarget.value || null })}>
      <option value="">All languages</option>
      {#each store.languages as language (language)}
        <option value={language}>{language}</option>
      {/each}
    </Select>
  </label>
  <label>
    Size
    <Select value={size} onchange={(event) => chooseSize(event.currentTarget.value)}>
      {#each SIZES as range, index (range.label)}
        <option value={index}>{range.label}</option>
      {/each}
    </Select>
  </label>
</div>

<style>
  .browse {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3) var(--space-5);
  }

  label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
