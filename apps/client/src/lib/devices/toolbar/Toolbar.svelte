<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { DeviceSort } from '$lib/hub/generated/graphql';
  import { press } from '$lib/motion/roles/press';
  import type { ListQuery } from '../query';
  import Menu from './Menu.svelte';
  import { ALL_SITES, NAME_DEBOUNCE_MS, SORT_OPTIONS, debounced, siteButtonLabel, toggled, type MenuOption } from './toolbar';

  interface Props {
    query: ListQuery;
    tags: readonly string[];
    /** The text for an empty tag menu: why there are no tags. */
    tagsNote: string;
    /** The Site menu options: "All sites" and each site; none while there is nothing to choose (VPN-002). */
    sites: readonly MenuOption[];
    /** The text for an empty site menu: why there are no sites. */
    sitesNote: string;
    onQueryChange: (query: ListQuery) => void;
  }

  const { query, tags, tagsNote, sites, sitesNote, onQueryChange }: Props = $props();
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
    <Icon name="carbon:search" size="sm" />
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
    label="Tags"
    icon="carbon:tag"
    kind="checkbox"
    options={tagOptions}
    chosen={query.tags}
    empty={tagsNote}
    onChoose={(value) => {
      onQueryChange({ ...query, tags: toggled(query.tags, value) });
    }}
  />
  <Menu
    label="Site"
    shown={siteButtonLabel(sites, query.site)}
    icon="carbon:location"
    kind="radio"
    options={sites}
    chosen={[query.site ?? ALL_SITES]}
    empty={sitesNote}
    onChoose={(value) => {
      onQueryChange({ ...query, site: value === ALL_SITES ? null : value });
    }}
  />
  <span class="spacer"></span>
  <Menu
    label="Sort"
    shown={SORT_OPTIONS.find((option) => option.value === query.sort)?.label.split(',')[0] ?? 'Sort'}
    icon="carbon:arrows-vertical"
    kind="radio"
    options={SORT_OPTIONS}
    chosen={[query.sort]}
    onChoose={(value) => {
      onQueryChange({ ...query, sort: value as DeviceSort });
    }}
  />
  <div class="views" role="group" aria-label="View">
    {#each [{ view: 'table', label: 'List', icon: 'carbon:list' }, { view: 'grid', label: 'Grid', icon: 'carbon:grid' }] as const as option (option.view)}
      <button
        type="button"
        tabindex="0"
        aria-pressed={query.view === option.view}
        aria-label={option.label}
        title={option.label}
        onclick={() => {
          onQueryChange({ ...query, view: option.view });
        }}
        {@attach press()}><Icon name={option.icon} size="sm" /></button
      >
    {/each}
  </div>
</div>

<style>
  /* One quiet toolbar between hairlines (UR-7, D18): search, Tags and Site; then Sort and the view. */
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1) var(--space-2);
    padding: var(--space-2) 0;
    border-block: var(--border-thin) solid var(--border-subtle);
  }

  .spacer {
    flex: 1;
  }

  .search {
    flex: 0 1 calc(var(--space-16) * 4);
    min-inline-size: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--control-md);
    padding: 0 var(--space-2) 0 var(--space-1);
    border: var(--border-thin) solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-tertiary);
    transition: border-color var(--duration-fast) var(--ease-enter), background-color var(--duration-fast) var(--ease-enter);
  }

  .search:focus-within {
    border-color: var(--accent);
    background: var(--surface-base);
  }

  .label {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  input::placeholder {
    color: var(--text-tertiary);
  }

  input {
    flex: 1;
    align-self: stretch;
    min-inline-size: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    outline: none;
  }

  .views {
    display: inline-flex;
    padding: var(--border-focus);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
  }

  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: var(--control-md);
    block-size: calc(var(--control-md) - var(--space-2));
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-tertiary);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter), color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    color: var(--text);
  }

  button[aria-pressed='true'] {
    background: var(--surface-raised);
    color: var(--text);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }

  /* On a phone the search takes its own line, above the menus. */
  @media (max-width: 767.98px) {
    .search {
      flex: 1 1 100%;
      order: -1;
      border-color: var(--border-subtle);
    }
  }
</style>
