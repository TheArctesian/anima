<script lang="ts">
  import { HeldOrder } from '$lib/devices/list/held.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { reorderMotion } from '$lib/motion/roles/reorder.svelte';
  import QuietLabel from '$lib/page/QuietLabel.svelte';
  import { EARLIER_KEY, orderedThreads, STATE_MARKS, type Seen, type StatedRow } from '../state';
  import { ago, folderName, programName, type Program, type ThreadRow } from '../thread';

  type Choice = 'all' | Program;

  interface Props {
    /** The rows after the search and the choice of a program. */
    rows: readonly ThreadRow[];
    chosen: string | null;
    program: Program | null;
    search: string;
    now: Date;
    /** When the person last opened each thread. */
    seen: Seen;
    /** Whether the group "Earlier" shows its threads; it is folded at first, and a search opens it (THR-056). */
    earlierOpen: boolean;
    onToggleEarlier: () => void;
    onChoose: (id: string) => void;
    onProgram: (program: Program | null) => void;
    onSearch: (search: string) => void;
    /** The search field, so that the key `/` can go to it. */
    field?: HTMLInputElement | undefined;
  }

  let { rows, chosen, program, search, now, seen, earlierOpen, onToggleEarlier, onChoose, onProgram, onSearch, field = $bindable() }: Props = $props();
  const choice = $derived<Choice>(program ?? 'all');
  const OPTIONS: readonly { value: Choice; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'claude_code', label: 'Claude Code' },
    { value: 'codex', label: 'Codex' },
  ];
  /** While the pointer is on the list a row keeps its place; a search or a choice of a program applies at once (5.7.3, Reorder). */
  const held = new HeldOrder<StatedRow>(
    () => orderedThreads(rows, now, seen, { earlierOpen, chosen }),
    () => `${program ?? 'all'}\n${search}\n${String(earlierOpen)}\n${chosen ?? ''}`,
  );
  const items = $derived(held.items);
  const track = reorderMotion(() => items.map((item) => item.key).join('\n'));
</script>

<!-- The threads of this computer (THR-013, THR-056): those that need the person at the top, those that work at the bottom,
     the others between, in a group "Earlier" that is folded at first. A row is a button: it opens its thread beside the list. Each row has the state that its session
     file gives, as an icon with a word. While the pointer is on the list no row moves. -->
<div class="list">
  <label class="search">
    <Icon name="carbon:search" size="sm" />
    <input
      bind:this={field}
      type="search"
      placeholder="Search threads"
      aria-label="Search threads"
      value={search}
      oninput={(event) => {
        onSearch(event.currentTarget.value);
      }}
    />
    <kbd>/</kbd>
  </label>
  <Segmented
    label="Program"
    size="sm"
    options={OPTIONS}
    value={choice}
    onchange={(value) => {
      onProgram(value === 'all' ? null : value);
    }}
  />
  <!-- The rows scroll on their own in a wide window, so the search and the filter stay in view. -->
  <ul {@attach held.hold}>
    {#each items as item (item.key)}
      {#if item.kind === 'group'}
        <li class="head" {@attach track(item.key)}>
          {#if item.key === EARLIER_KEY}
            <button type="button" class="fold" aria-expanded={earlierOpen} onclick={onToggleEarlier}>
              <Icon name={earlierOpen ? 'carbon:chevron-down' : 'carbon:chevron-right'} size="sm" />
              <QuietLabel>{item.title}</QuietLabel>
            </button>
          {:else}
            <QuietLabel>{item.title}</QuietLabel>
          {/if}
          <span class="count">{item.count}</span>
        </li>
      {:else}
        {@const row = item.device.row}
        {@const mark = STATE_MARKS[item.device.state]}
        <li {@attach track(item.key)}>
          <button type="button" class="row" aria-current={row.id === chosen ? 'true' : undefined} data-thread={row.id} data-state={item.device.state} onclick={() => { onChoose(row.id); }}>
            <span class="title"><span class="words">{row.title ?? 'Untitled thread'}</span></span>
            <span class="facts">
              <span class="state" title={mark.detail}><Icon name={mark.icon} size="sm" />{mark.words}</span>
              <span>{programName(row.program)}</span>
              <span class="folder">{folderName(row.folder)}</span>
              <span class="ago">{ago(row.last, now)}</span>
            </span>
          </button>
        </li>
      {/if}
    {:else}
      <li class="none">{search === '' && program === null ? 'No session of Claude Code or Codex is on this computer.' : 'No thread has these words.'}</li>
    {/each}
  </ul>
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    min-block-size: 0;
  }

  .search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--control-md);
    padding-inline: var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text-secondary);
  }

  .search:focus-within {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: 0;
  }

  input {
    flex: 1;
    min-inline-size: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--text);
    font: inherit;
  }

  kbd {
    padding-inline: var(--space-1);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .head {
    display: flex;
    justify-content: space-between;
    padding: var(--space-2) var(--space-2) 0;
  }

  .fold {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-secondary);
    font: inherit;
    cursor: pointer;
  }

  .fold:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--space-1);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    white-space: nowrap;
  }

  [data-state='asks'] .state,
  [data-state='may_wait'] .state,
  [data-state='your_turn'] .state {
    color: var(--status-degraded-text);
    font-weight: var(--weight-medium);
  }

  [data-state='working'] .state {
    color: var(--text);
  }

  .count,
  .ago {
    color: var(--text-secondary);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  ul {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-1);
    min-block-size: 0;
    margin: 0;
    /* Room for the focus ring of a row, which the scroll area would cut. */
    padding: var(--border-focus);
    overflow-y: auto;
    overscroll-behavior: contain;
    list-style: none;
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    inline-size: 100%;
    padding: var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text);
    font: inherit;
    text-align: start;
    cursor: pointer;
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .row[aria-current='true'] {
    background: var(--surface-selected);
  }

  .row:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: 0;
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .words,
  .folder {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .words {
    flex: 1;
    min-inline-size: 0;
  }

  .facts {
    display: flex;
    gap: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  .folder {
    font-family: var(--font-mono);
  }

  .ago {
    margin-inline-start: auto;
    white-space: nowrap;
  }

  .none {
    margin: 0;
    padding: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
