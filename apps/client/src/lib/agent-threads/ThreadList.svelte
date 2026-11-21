<script lang="ts">
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import QuietLabel from '$lib/page/QuietLabel.svelte';
  import { ago, changedNow, folderName, groups, programName, type Program, type ThreadRow } from './thread';

  type Choice = 'all' | Program;

  interface Props {
    /** The rows after the search and the choice of a program, the last changed first. */
    rows: readonly ThreadRow[];
    chosen: string | null;
    program: Program | null;
    search: string;
    now: Date;
    onChoose: (id: string) => void;
    onProgram: (program: Program | null) => void;
    onSearch: (search: string) => void;
    /** The search field, so that the key `/` can go to it. */
    field?: HTMLInputElement | undefined;
  }

  let { rows, chosen, program, search, now, onChoose, onProgram, onSearch, field = $bindable() }: Props = $props();
  const choice = $derived<Choice>(program ?? 'all');
  const OPTIONS: readonly { value: Choice; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'claude_code', label: 'Claude Code' },
    { value: 'codex', label: 'Codex' },
  ];
</script>

<!-- The threads of this computer in three groups, the last changed first (THR-013). A row is a button: it opens its
     thread beside the list. The dot marks a file that changed in the last 2 minutes; it is not a state of the agent. -->
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
  <!-- The groups scroll on their own in a wide window, so the search and the filter stay in view. -->
  <div class="groups">
    {#each groups(rows, now) as group (group.name)}
      <section aria-label={group.name}>
        <div class="head"><QuietLabel>{group.name}</QuietLabel><span class="count">{group.rows.length}</span></div>
        <ul>
          {#each group.rows as row (row.id)}
            <li>
              <button type="button" class="row" aria-current={row.id === chosen ? 'true' : undefined} data-thread={row.id} onclick={() => { onChoose(row.id); }}>
                <span class="title">
                  <span class="dot" class:live={changedNow(row, now)} title={changedNow(row, now) ? 'Changed in the last 2 minutes' : undefined}></span>
                  <span class="words">{row.title ?? 'Untitled thread'}</span>
                </span>
                <span class="facts">
                  <span>{programName(row.program)}</span>
                  <span class="folder">{folderName(row.folder)}</span>
                  <span class="ago">{ago(row.last, now)}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {:else}
      <p class="none">{search === '' && program === null ? 'No session of Claude Code or Codex is on this computer.' : 'No thread has these words.'}</p>
    {/each}
  </div>
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    min-block-size: 0;
  }

  .groups {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-3);
    min-block-size: 0;
    /* Room for the focus ring of a row, which the scroll area would cut. */
    padding: var(--border-focus);
    overflow-y: auto;
    overscroll-behavior: contain;
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
    padding-inline: var(--space-2);
  }

  .count,
  .ago {
    color: var(--text-secondary);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
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

  .dot {
    flex: none;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-unknown);
  }

  .dot.live {
    background: var(--accent);
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
