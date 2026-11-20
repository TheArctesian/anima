<script lang="ts">
  import type { UsageSums } from '../api';
  import { costText, leftOutLine } from '../report/words';
  import { whole } from '../report/units';
  import { lastUseWords, sharedHost, type RepositoryLine } from './shape';

  interface Props {
    lines: readonly RepositoryLine[];
    /** The total with no repository chosen: the first line. */
    everything: UsageSums;
    /** How many days the page shows. */
    days: number;
    /** The chosen repository; null for all. */
    chosen: string | null;
    /** The UTC day of now, `YYYY-MM-DD`. */
    today: string;
    /** How many repositories the hub left out. */
    leftOut: number;
    onChoose: (repository: string | null) => void;
  }

  const { lines, everything, days, chosen, today, leftOut, onChoose }: Props = $props();
  const more = $derived(leftOutLine(leftOut));
  /* When each repository is on one host the list leaves the host out; the tooltip has the whole name. */
  const host = $derived(sharedHost(lines));
</script>

<!-- The repositories with use, newest use first. A line limits the page to its repository; the use that has no
     repository cannot be asked for alone (the hub's filter names a repository), so its line is not a choice. -->
<nav class="repositories" aria-label="Repositories">
  <span class="label">Repositories, newest use first</span>
  <button type="button" class="line" aria-current={chosen === null ? 'true' : undefined} onclick={() => { onChoose(null); }}>
    <span class="what"><span class="name">All repositories</span><span class="when">{days} days</span></span>
    <span class="counts"><span>{whole(everything.sessions)}</span><span class="cost">{costText(everything)}</span></span>
  </button>
  {#each lines as line (line.repository ?? '')}
    {@const when = lastUseWords(line.lastDay, today)}
    {#if line.repository === null}
      <div class="line">
        <span class="what"><span class="name">No repository</span><span class="when">{when}</span></span>
        <span class="counts"><span>{whole(line.sessions)}</span><span class="cost">{costText(line)}</span></span>
      </div>
    {:else}
      {@const repository = line.repository}
      <button type="button" class="line" aria-current={chosen === repository ? 'true' : undefined} onclick={() => { onChoose(repository); }}>
        <span class="what"><span class="name" title={repository}>{repository.slice(host.length)}</span><span class="when">{when}</span></span>
        <span class="counts"><span>{whole(line.sessions)}</span><span class="cost">{costText(line)}</span></span>
      </button>
    {/if}
  {/each}
  {#if more !== ''}<span class="more">{more}</span>{/if}
</nav>

<style>
  .repositories {
    display: flex;
    flex-direction: column;
    gap: var(--space-0);
    min-inline-size: 0;
  }

  .label {
    padding: 0 var(--space-2) var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
  }

  .line {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    box-sizing: border-box;
    inline-size: 100%;
    padding: var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: start;
  }

  button.line {
    cursor: pointer;
  }

  button.line:hover {
    background: var(--surface-hover);
  }

  button.line[aria-current='true'] {
    background: var(--surface-selected);
  }

  button.line:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(-1 * var(--border-focus));
  }

  .what {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-0);
    min-inline-size: 0;
  }

  .name {
    overflow: hidden;
    font-family: var(--font-mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .when,
  .cost,
  .more {
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  .counts {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .more {
    padding: var(--space-2);
    line-height: var(--leading-normal);
  }
</style>
