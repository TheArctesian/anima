<script lang="ts">
  import { untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import { dataAge } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import ForgeSection from '../forge/ForgeSection.svelte';
  import type { ForgeApi } from '../forge/api';
  import { ForgeList } from '../forge/list.svelte';
  import type { Issue, OpenOrClosed } from '../forge/types';
  import IssueDetail from './IssueDetail.svelte';
  import NewIssueDialog from './NewIssueDialog.svelte';

  interface Props {
    api: ForgeApi;
    repository: string;
    now: () => Date;
  }

  const { api, repository, now }: Props = $props();
  let showing = $state<OpenOrClosed>('OPEN');
  const list = new ForgeList<Issue>((ask) => api.issues(repository, showing, ask), (row) => String(row.number));
  let shown = $state<number | null>(null);
  let creating = $state(false);

  $effect(() => {
    void showing;
    shown = null;
    untrack(() => void list.load());
  });

  /** A changed issue stays in the list only while it still matches Open or Closed. */
  function changed(issue: Issue): void {
    list.put(issue, issue.open === (showing === 'OPEN'));
  }
</script>

{#snippet tools()}
  <Segmented label="Issues to show" size="sm" options={[{ value: 'OPEN', label: 'Open' }, { value: 'CLOSED', label: 'Closed' }]} bind:value={showing} />
  <span class="end"><FormButton size="sm" onclick={() => (creating = true)}><Icon name="carbon:add" size="sm" />New issue</FormButton></span>
{/snippet}

<ForgeSection title="Issues" icon="carbon:circle-dash" {list} {now} {tools} empty={showing === 'OPEN' ? 'No open issues' : 'No closed issues'}>
  <ul class="rows" aria-label="Issues">
    {#each list.rows as issue (issue.number)}
      <li class="row">
        <span class="mark" class:closed={!issue.open}>
          <Icon name={issue.open ? 'carbon:circle-dash' : 'carbon:checkmark-outline'} size="sm" />
        </span>
        <span class="main">
          <button
            type="button"
            class="title"
            aria-expanded={shown === issue.number}
            onclick={() => (shown = shown === issue.number ? null : issue.number)}>{issue.title}</button
          >
          <span class="sub">
            <span class="mono">#{issue.number}</span> · {issue.authorLogin} · {dataAge(new Date(issue.updatedAt), now())}
            {#if issue.comments > 0}· {issue.comments} {issue.comments === 1 ? 'comment' : 'comments'}{/if}
          </span>
        </span>
        <span class="labels">{#each issue.labels as label (label)}<span class="label">{label}</span>{/each}</span>
        {#if shown === issue.number}
          <div class="detail"><IssueDetail {api} {issue} {now} onChange={changed} /></div>
        {/if}
      </li>
    {/each}
  </ul>
</ForgeSection>

{#if creating}
  <NewIssueDialog
    {api}
    {repository}
    onDone={(issue) => {
      creating = false;
      if (showing === 'OPEN') list.put(issue);
      shown = issue.number;
    }}
    onCancel={() => (creating = false)}
  />
{/if}

<style>
  .end {
    margin-inline-start: auto;
  }

  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: grid;
    grid-template-columns: var(--icon-sm) minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-2) 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .mark {
    display: inline-flex;
    color: var(--status-up);
  }

  .mark.closed {
    color: var(--text-quiet);
  }

  .main {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }

  .title {
    overflow: hidden;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: start;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title:hover {
    text-decoration: underline;
  }

  .title:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .sub {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .labels {
    display: flex;
    gap: var(--space-1);
  }

  .label {
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  .detail {
    grid-column: 2 / -1;
  }
</style>
