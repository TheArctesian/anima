<script lang="ts">
  import { untrack } from 'svelte';
  import { dayLabel } from '$lib/events/panel/view';
  import { dataAge } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import BranchPick from '../forge/BranchPick.svelte';
  import ForgeSection from '../forge/ForgeSection.svelte';
  import type { ForgeApi } from '../forge/api';
  import { ForgeList } from '../forge/list.svelte';
  import type { Commit } from '../forge/types';
  import { checksMark, shortSha } from '../forge/words';

  interface Props {
    api: ForgeApi;
    repository: string;
    defaultBranch: string;
    now: () => Date;
  }

  const { api, repository, defaultBranch, now }: Props = $props();
  // svelte-ignore state_referenced_locally
  let branch = $state(defaultBranch);
  const list = new ForgeList<Commit>((ask) => api.commits(repository, branch, ask), (row) => row.sha);

  $effect(() => {
    void branch;
    untrack(() => void list.load());
  });

  /** Each commit with the day heading it opens, if it is the first of its day. */
  const days = $derived(
    list.rows.map((commit, index) => {
      const day = dayLabel(commit.committedAt, now());
      const before = list.rows[index - 1];
      return before !== undefined && dayLabel(before.committedAt, now()) === day ? null : day;
    }),
  );
</script>

{#snippet tools()}
  <BranchPick {api} {repository} value={branch} onChange={(next) => (branch = next)} />
{/snippet}

<ForgeSection title="Commits" icon="carbon:recently-viewed" {list} {now} {tools} empty="No commits">
  <ol class="rows" aria-label="Commits">
    {#each list.rows as commit, index (commit.sha)}
      {@const mark = checksMark(commit.checks)}
      {@const day = days[index]}
      {#if day !== null && day !== undefined && day !== ''}<li class="day" aria-hidden="true">{day}</li>{/if}
      <li class="row">
        <span class="mark {mark.tone}" title={mark.words}><Icon name={mark.icon} size="sm" /><span class="hidden">{mark.words}</span></span>
        <span class="main">
          <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub -->
          <a href={commit.url} target="_blank" rel="noreferrer">{commit.title}</a>
          <span class="sub">{commit.authorLogin ?? commit.authorName} · {dataAge(new Date(commit.committedAt), now())}</span>
        </span>
        <span class="sha">{shortSha(commit.sha)}</span>
      </li>
    {/each}
  </ol>
</ForgeSection>

<style>
  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .day {
    padding: var(--space-3) 0 var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
  }

  .row {
    display: grid;
    grid-template-columns: var(--icon-sm) minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .mark {
    display: inline-flex;
  }

  .hidden {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .main {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }

  .main a {
    overflow: hidden;
    color: var(--text);
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .main a:hover {
    text-decoration: underline;
  }

  .sub {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }

  .sha {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .up {
    color: var(--status-up);
  }

  .down {
    color: var(--status-down);
  }

  .running {
    color: var(--accent);
  }

  .quiet {
    color: var(--text-quiet);
  }
</style>
