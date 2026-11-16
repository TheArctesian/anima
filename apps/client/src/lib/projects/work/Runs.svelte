<script lang="ts">
  import { untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { dataAge } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import ChangeDialog from '../forge/ChangeDialog.svelte';
  import ForgeSection from '../forge/ForgeSection.svelte';
  import type { ForgeApi, RunChange } from '../forge/api';
  import { ForgeList } from '../forge/list.svelte';
  import type { WorkflowRun } from '../forge/types';
  import { runState, running, shortSha } from '../forge/words';
  import DispatchDialog from './DispatchDialog.svelte';

  interface Props {
    api: ForgeApi;
    repository: string;
    /** The branch a workflow starts on by default. */
    defaultBranch: string;
    now: () => Date;
  }

  const { api, repository, defaultBranch, now }: Props = $props();
  /** How often the runs are read again while one is queued or running; there is no push for them (PRJ-034). */
  const EVERY_MS = 30_000;
  const list = new ForgeList<WorkflowRun>((ask) => api.workflowRuns(repository, ask), (row) => row.id);
  let asking = $state.raw<{ change: RunChange; run: WorkflowRun } | null>(null);
  let dispatching = $state(false);
  const going = $derived(list.rows.some(running));

  $effect(() => {
    untrack(() => void list.load());
  });

  $effect(() => {
    if (!going) return;
    const timer = setInterval(() => void list.load(true), EVERY_MS);
    return () => {
      clearInterval(timer);
    };
  });

  const CHANGE_WORDS: Record<RunChange, { title: string; body: string; label: string }> = {
    rerunFailed: { title: 'Re-run failed jobs', body: 'Re-runs the failed jobs with the repository’s secrets.', label: 'Re-run failed jobs' },
    rerun: { title: 'Re-run every job', body: 'Re-runs every job with the repository’s secrets.', label: 'Re-run' },
    cancel: { title: 'Cancel run', body: 'Stops unfinished jobs.', label: 'Cancel run' },
  };
</script>

{#snippet tools()}
  <span class="end">
    <FormButton size="sm" tone="ghost" onclick={() => (dispatching = true)}><Icon name="carbon:play" size="sm" />Run workflow</FormButton>
  </span>
{/snippet}

<ForgeSection title="Workflow runs" icon="carbon:play" {list} {now} {tools} empty="No workflow runs">
  <ul class="rows" aria-label="Workflow runs">
    {#each list.rows as run (run.id)}
      {@const mark = runState(run)}
      <li class="row" class:failed={mark.tone === 'down'}>
        <span class="mark {mark.tone}"><Icon name={mark.icon} size="sm" /></span>
        <span class="main">
          <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub -->
          <a href={run.url} target="_blank" rel="noreferrer">{run.name} #{run.runNumber}</a>
          <span class="sub">
            {#if run.branch}<span class="mono">{run.branch}</span> · {/if}<span class="mono">{shortSha(run.headSha)}</span>
            {#if run.commitTitle}· {run.commitTitle}{/if} · {run.event} · {dataAge(new Date(run.createdAt), now())}
          </span>
        </span>
        <span class="state {mark.tone}">{mark.words}</span>
        <span class="actions">
          {#if run.failedJobsRerunnable}
            <FormButton size="sm" onclick={() => (asking = { change: 'rerunFailed', run })} aria-label="Re-run failed jobs of {run.name} #{run.runNumber}">Re-run failed</FormButton>
          {/if}
          {#if run.rerunnable}
            <FormButton size="sm" tone="ghost" onclick={() => (asking = { change: 'rerun', run })} aria-label="Re-run {run.name} #{run.runNumber}">Re-run</FormButton>
          {/if}
          {#if run.cancellable}
            <FormButton size="sm" tone="ghost" onclick={() => (asking = { change: 'cancel', run })} aria-label="Cancel {run.name} #{run.runNumber}">Cancel</FormButton>
          {/if}
          {#if run.fromFork && run.status === 'COMPLETED'}<span class="sub">From a fork: re-run it on GitHub</span>{/if}
        </span>
      </li>
    {/each}
  </ul>
</ForgeSection>

{#if asking !== null}
  {@const words = CHANGE_WORDS[asking.change]}
  {@const target = asking}
  <ChangeDialog
    open
    title={words.title}
    body={words.body}
    lines={[`${target.run.name} #${String(target.run.runNumber)}`, `Commit ${target.run.headSha}`, ...(target.run.branch ? [`Branch ${target.run.branch}`] : [])]}
    confirmLabel={words.label}
    tone={target.change === 'cancel' ? 'danger' : 'primary'}
    onConfirm={async () => {
      const result = await api.runChange(target.change, target.run);
      if (result.ok) list.put(result.value);
      return result;
    }}
    onDone={() => (asking = null)}
    onCancel={() => (asking = null)}
  />
{/if}

{#if dispatching}
  <DispatchDialog
    {api}
    {repository}
    {defaultBranch}
    onDone={() => {
      dispatching = false;
      void list.load(true);
    }}
    onCancel={() => (dispatching = false)}
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
    grid-template-columns: var(--icon-sm) minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .row.failed {
    box-shadow: inset var(--border-focus) 0 0 var(--status-down);
    padding-inline-start: var(--space-2);
  }

  .mark {
    display: inline-flex;
  }

  .main {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }

  .main a {
    color: var(--text);
    text-decoration: none;
  }

  .main a:hover {
    text-decoration: underline;
  }

  .sub {
    overflow: hidden;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .state {
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    gap: var(--space-1);
  }

  .up {
    color: var(--status-up-text);
  }

  .down {
    color: var(--status-down-text);
  }

  .running {
    color: var(--accent);
  }

  .warning {
    color: var(--status-warning-text);
  }

  .quiet {
    color: var(--text-tertiary);
  }
</style>
