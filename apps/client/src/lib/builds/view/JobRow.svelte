<script lang="ts">
  import type { Builder } from '../api';
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { JobView } from './api';
  import { canCancel, stateLabel, targetLabel, timeInState, waitingText } from './release';

  interface Props {
    job: JobView;
    /** The version of the release the job belongs to; it names the job's controls. */
    version: string;
    builders: readonly Builder[];
    now: Date;
    selected: boolean;
    /** The hub's refusal of the last cancel of this job. */
    problem?: string | undefined;
    onselect: () => void;
    oncancel: () => void;
  }

  const { job, version, builders, now, selected, problem, onselect, oncancel }: Props = $props();
  const waiting = $derived(waitingText(job, builders));
  const target = $derived(targetLabel(job.target));
</script>

<li class:selected>
  <button class="pick" type="button" tabindex="0" aria-pressed={selected} aria-label="Show log of {target} job of {version}" onclick={onselect}>
    <strong>{target}</strong>
    <span class="state" data-state={job.state}>{stateLabel(job.state)}</span>
    <span>{timeInState(job, now)} in this state</span>
    <span>{job.attempts === 1 ? '1 attempt' : `${String(job.attempts)} attempts`}</span>
    {#if job.builderName}<span>Builder {job.builderName}</span>{/if}
  </button>
  {#if waiting !== null}<p class="note">{waiting}</p>{/if}
  {#if job.state === 'FAILED' && job.failureReason}<p class="note failure">{job.failureReason}</p>{/if}
  {#if canCancel(job)}
    <FormButton tabindex={0} size="sm" tone="danger" aria-label="Cancel {target} job of {version}" onclick={oncancel}>Cancel</FormButton>
  {/if}
  {#if problem}<p class="note failure" role="alert">{problem}</p>{/if}
</li>

<style>
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-md);
    list-style: none;
  }

  li.selected {
    background: var(--surface-selected);
  }

  .pick {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    gap: var(--space-3);
    align-items: center;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: start;
    cursor: pointer;
  }

  .pick:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .state {
    font-weight: var(--weight-bold);
  }

  .state[data-state='FAILED'] {
    color: var(--status-down-text);
  }

  .state[data-state='PUBLISHED'] {
    color: var(--status-up-text);
  }

  .note {
    flex-basis: 100%;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .failure {
    color: var(--status-down-text);
  }
</style>
