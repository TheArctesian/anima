<script lang="ts">
  import { untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import { dataAge } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import ForgeSection from '../forge/ForgeSection.svelte';
  import type { ForgeApi } from '../forge/api';
  import { ForgeList } from '../forge/list.svelte';
  import type { MergeOutcome, OpenOrClosed, PullRequest, PullRequestMerge, PullRequestPage } from '../forge/types';
  import { MERGE_STATE_WORDS, checksMark } from '../forge/words';
  import MergeDialog from './MergeDialog.svelte';

  interface Props {
    api: ForgeApi;
    repository: string;
    now: () => Date;
  }

  const { api, repository, now }: Props = $props();
  let showing = $state<OpenOrClosed>('OPEN');
  const list = new ForgeList<PullRequest, PullRequestPage>((ask) => api.pullRequests(repository, showing, ask), (row) => String(row.number));
  let selected = $state.raw<readonly number[]>([]);
  /** One pull request to merge alone, or the selected ones in bulk. */
  let merging = $state.raw<readonly PullRequest[] | null>(null);
  let results = $state.raw<readonly PullRequestMerge[]>([]);

  $effect(() => {
    void showing;
    selected = [];
    untrack(() => void list.load());
  });

  /** Only a clean pull request from this repository merges in bulk; one from a fork merges alone (PRJ-033, PRO-013). */
  const bulk = (pull: PullRequest) => pull.state === 'OPEN' && !pull.fromFork && pull.mergeability?.canMerge === true;
  const chosen = $derived(list.rows.filter((pull) => selected.includes(pull.number)));
  const methods = $derived(list.first?.mergeMethods ?? []);
  const OK: readonly MergeOutcome[] = ['MERGED', 'ALREADY_MERGED'];

  function toggle(number: number): void {
    selected = selected.includes(number) ? selected.filter((each) => each !== number) : [...selected, number];
  }

  function merged(outcomes: readonly PullRequestMerge[]): void {
    results = outcomes;
    selected = [];
    merging = null;
    void list.load(true);
  }
</script>

{#snippet tools()}
  <Segmented label="Pull requests to show" size="sm" options={[{ value: 'OPEN', label: 'Open' }, { value: 'CLOSED', label: 'Closed' }]} bind:value={showing} />
  {#if chosen.length > 0}
    <span class="picked">{chosen.length} selected</span>
    <FormButton tone="primary" size="sm" onclick={() => (merging = chosen)}><Icon name="carbon:merge" size="sm" />Merge {chosen.length}</FormButton>
  {/if}
{/snippet}

<ForgeSection title="Pull requests" icon="carbon:branch" {list} {now} {tools} empty={showing === 'OPEN' ? 'No open pull requests' : 'No closed pull requests'}>
  {#if results.length > 0}
    <ul class="results" aria-label="Merge results">
      {#each results as result (result.number)}
        <li class:bad={!OK.includes(result.outcome)}><span class="mono">#{result.number}</span>{result.message}</li>
      {/each}
    </ul>
  {/if}
  <ul class="rows" aria-label="Pull requests">
    {#each list.rows as pull (pull.number)}
      {@const mark = checksMark(pull.checks)}
      <li class="row">
        {#if pull.state === 'OPEN'}
          <input
            type="checkbox"
            aria-label="Select #{pull.number} to merge"
            disabled={!bulk(pull)}
            title={bulk(pull) ? undefined : pull.fromFork ? 'From a fork: merge it alone' : 'Not ready to merge'}
            checked={selected.includes(pull.number)}
            onchange={() => toggle(pull.number)}
          />
        {:else}<span></span>{/if}
        <span class="mark {mark.tone}" title={mark.words}><Icon name={mark.icon} size="sm" /></span>
        <span class="main">
          <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub -->
          <a href={pull.url} target="_blank" rel="noreferrer">{pull.title}</a>
          <span class="sub">
            <span class="mono">#{pull.number}</span> · {pull.authorLogin} · <span class="mono">{pull.headBranch}</span>
            {#if pull.fromFork}· from {pull.headRepository ?? 'a deleted fork'}{/if} · {dataAge(new Date(pull.updatedAt), now())}
          </span>
        </span>
        <span class="state">
          {#if pull.state !== 'OPEN'}{pull.state === 'MERGED' ? 'Merged' : 'Closed'}
          {:else if pull.draft}Draft
          {:else}{pull.mergeability ? MERGE_STATE_WORDS[pull.mergeability.state] : mark.words}{/if}
        </span>
        {#if pull.state === 'OPEN' && (pull.mergeability?.canMerge === true || pull.mergeability?.state === 'UNSTABLE')}
          <FormButton tone="ghost" size="sm" onclick={() => (merging = [pull])} aria-label="Merge #{pull.number}">Merge</FormButton>
        {:else}<span></span>{/if}
      </li>
    {/each}
  </ul>
</ForgeSection>

{#if merging !== null}
  <MergeDialog {api} pulls={merging} {methods} onDone={merged} onCancel={() => (merging = null)} />
{/if}

<style>
  .picked {
    margin-inline-start: auto;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  .rows,
  .results {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .results {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }

  .results li {
    display: flex;
    gap: var(--space-2);
    color: var(--status-up-text);
  }

  .results li.bad {
    color: var(--status-down-text);
  }

  .row {
    display: grid;
    grid-template-columns: var(--control-sm) var(--icon-sm) minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
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

  .sub,
  .state {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .mark {
    display: inline-flex;
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
