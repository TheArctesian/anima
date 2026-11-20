<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import { saveBlob, type BlobSaver } from '$lib/sessions/files/save';
  import type { ContainerLog } from '../api';
  import { LEVEL_CHOICES, LEVEL_LABELS, downloadName, downloadText, lineTime, shownLines, type LevelChoice } from './log';

  /**
   * HOST-001: the last lines of one container, oldest first, as text. The lines are the container's own: never drawn as
   * markup. HOST-015: a search, a filter by level, a switch for line wrap, a switch for timestamps and a download of the
   * lines that are shown. All of it is work on the lines the hub gave; nothing is kept.
   */
  const { log, container, save = saveBlob }: { log: ContainerLog; container: string; save?: BlobSaver } = $props();

  let search = $state('');
  let level = $state<LevelChoice>('all');
  let wrap = $state(true);
  let timestamps = $state(true);

  const levelOptions = LEVEL_CHOICES.map((value) => ({ value, label: LEVEL_LABELS[value] }));
  const lines = $derived(shownLines(log.lines, { search, level }));

  function download(): void {
    save(downloadName(container), new Blob([downloadText(lines, timestamps)], { type: 'text/plain' }));
  }
</script>

{#if log.cut}
  <p class="cut" role="status">The log passed 4 MiB: the newest lines are missing. Choose fewer lines.</p>
{/if}
{#if log.lines.length === 0}
  <p class="none">The container wrote no line.</p>
{:else}
  <div class="tools">
    <div class="search"><Input type="search" icon="carbon:search" aria-label="Search the log" placeholder="Search" bind:value={search} /></div>
    <Segmented label="Level" size="sm" options={levelOptions} bind:value={level} />
    <label class="switch"><Switch label="Wrap lines" bind:checked={wrap} /><span aria-hidden="true">Wrap</span></label>
    <label class="switch"><Switch label="Show timestamps" bind:checked={timestamps} /><span aria-hidden="true">Timestamps</span></label>
    <!-- The count is in the bar of the tools, so the lines start one row higher (R-224). -->
    <p class="shown" role="status">{lines.length} of {log.lines.length} {log.lines.length === 1 ? 'line' : 'lines'}</p>
    <FormButton size="sm" tone="ghost" icon="carbon:download" aria-disabled={lines.length === 0} onclick={() => { if (lines.length > 0) download(); }}>Download</FormButton>
  </div>
  {#if lines.length === 0}
    <p class="none">No line has this text and this level.</p>
  {:else}
    <!-- A focusable scroll region, so the keyboard can scroll it (WCAG 2.1.1). -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <ol class="lines" class:nowrap={!wrap} tabindex="0" aria-label="Log lines, oldest first">
      {#each lines as line, index (index)}
        <li>{#if timestamps}<time datetime={line.at ?? undefined}>{lineTime(line.at)}</time>{/if}<span>{line.text}</span></li>
      {/each}
    </ol>
  {/if}
{/if}

<style>
  .cut,
  .none,
  .shown {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .shown {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .search {
    flex: 1;
    min-inline-size: 12rem;
  }

  .switch {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .lines {
    box-sizing: border-box;
    max-block-size: 60vh;
    margin: 0;
    padding: var(--space-2) var(--space-3);
    overflow: auto;
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    list-style: none;
  }

  .lines:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  li {
    display: flex;
    gap: var(--space-3);
  }

  time {
    flex: none;
    color: var(--text-secondary);
    user-select: none;
  }

  span {
    min-inline-size: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  /* With no wrap a long line keeps its one row and the region scrolls sideways. */
  .nowrap span {
    white-space: pre;
    overflow-wrap: normal;
  }
</style>
