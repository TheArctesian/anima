<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { DeviceSort, Status } from '$lib/hub/generated/graphql';
  import { press } from '$lib/motion/roles/press';
  import type { ListQuery } from '../query';
  import Menu from './Menu.svelte';
  import { NAME_DEBOUNCE_MS, SORT_OPTIONS, STATUS_OPTIONS, debounced, toggled } from './toolbar';

  interface Props {
    query: ListQuery;
    tags: readonly string[];
    /** The text for an empty tag menu: why there are no tags. */
    tagsNote: string;
    onQueryChange: (query: ListQuery) => void;
    onAddDevice: () => void;
  }

  const { query, tags, tagsNote, onQueryChange, onAddDevice }: Props = $props();
  const tagOptions = $derived(tags.map((tag) => ({ value: tag, label: tag })));
  let text = $state('');
  const send = debounced(() => {
    onQueryChange({ ...query, name: text });
  }, NAME_DEBOUNCE_MS);

  /** The URL is the truth: Back, Forward and "Clear filters" change the name from outside. */
  $effect(() => {
    text = query.name;
    send.cancel();
  });
  $effect(() => send.cancel);
</script>

<div class="toolbar" role="search" aria-label="Filter devices">
  <label class="search">
    <span class="label">Search by name</span>
    <Icon name="nf-md-magnify" size="sm" />
    <input
      type="search"
      placeholder="Search by name"
      value={text}
      oninput={(event) => {
        text = event.currentTarget.value;
        send();
      }}
    />
  </label>
  <Menu
    label="Status"
    icon="nf-md-filter_variant"
    kind="checkbox"
    options={STATUS_OPTIONS}
    chosen={query.statuses}
    onChoose={(value) => {
      onQueryChange({ ...query, statuses: toggled(query.statuses, value as Status) });
    }}
  />
  <Menu
    label="Tags"
    icon="nf-md-tag"
    kind="checkbox"
    options={tagOptions}
    chosen={query.tags}
    empty={tagsNote}
    onChoose={(value) => {
      onQueryChange({ ...query, tags: toggled(query.tags, value) });
    }}
  />
  <Menu
    label="Sort"
    icon="nf-md-sort"
    kind="radio"
    options={SORT_OPTIONS}
    chosen={[query.sort]}
    onChoose={(value) => {
      onQueryChange({ ...query, sort: value as DeviceSort });
    }}
  />
  <div class="views" role="group" aria-label="View">
    {#each [{ view: 'grid', label: 'Grid' }, { view: 'table', label: 'Table' }] as const as option (option.view)}
      <button
        type="button"
        tabindex="0"
        aria-pressed={query.view === option.view}
        onclick={() => {
          onQueryChange({ ...query, view: option.view });
        }}
        {@attach press()}>{option.label}</button
      >
    {/each}
  </div>
  <button type="button" class="add" tabindex="0" onclick={onAddDevice} {@attach press()}>
    <Icon name="nf-md-plus" size="sm" />
    <span>Add device</span>
  </button>
</div>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: calc(var(--target-min-desktop) + var(--space-2));
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-muted);
  }

  .label {
    position: absolute;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  input::placeholder {
    color: var(--text-muted);
  }

  input {
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    outline: none;
  }

  .search:focus-within {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .views {
    display: inline-flex;
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: calc(var(--target-min-desktop) + var(--space-2));
    padding: 0 var(--space-3);
    border: 0;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-raised);
  }

  button:active {
    background: var(--border);
  }

  button[aria-pressed='true'] {
    background: var(--accent-tint);
    color: var(--accent);
  }

  .add {
    margin-inline-start: auto;
    border: var(--border-thin) solid var(--accent);
    border-radius: var(--radius-md);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
