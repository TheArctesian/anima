<script lang="ts">
  import type { Builder } from '../../api';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { JobView } from '../api';
  import { canCancel, stateAge, stateLabel, targetLabel, waitingText } from '../release';
  import { jobStages, stateTone } from './progress';
  import StageTrack from './StageTrack.svelte';
  import StatePill from './StatePill.svelte';

  interface Props {
    job: JobView;
    /** The release the job belongs to, as its controls name it: "0.1.1", or "0.1.1, attempt 2". */
    version: string;
    builders: readonly Builder[] | null;
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
  /** A published or cancelled job's stages are all alike; its state already says so. */
  const tracked = $derived(job.state !== 'PUBLISHED' && job.state !== 'CANCELLED');
</script>

<li class="job" class:selected>
  <button class="pick" type="button" tabindex="0" aria-pressed={selected} aria-label="Show log of {target} job of {version}" onclick={onselect}>
    <span class="platform" aria-hidden="true"><Icon name={job.target === 'DARWIN_UNIVERSAL' ? 'nf-md-apple' : 'nf-md-linux'} /></span>
    <strong class="target">{target}</strong>
    <StatePill word={stateLabel(job.state)} tone={stateTone(job.state)} />
    <span class="facts">
      <span>{stateAge(job, now)}</span>
      {#if job.builderName}<span>on {job.builderName}</span>{/if}
      {#if job.attempts > 1}<span>{job.attempts} attempts</span>{/if}
    </span>
    <span class="open" aria-hidden="true"><Icon name="nf-md-console_line" size="sm" /></span>
  </button>
  {#if tracked}<StageTrack stages={jobStages(job)} />{/if}
  {#if waiting !== null}<p class="note">{waiting}</p>{/if}
  {#if job.state === 'FAILED' && job.failureReason}<p class="note failure">{job.failureReason}</p>{/if}
  {#if canCancel(job)}
    <div class="actions">
      <FormButton tabindex={0} size="sm" tone="danger" aria-label="Cancel {target} job of {version}" onclick={oncancel}>Cancel</FormButton>
    </div>
  {/if}
  {#if problem}<p class="note failure" role="alert">{problem}</p>{/if}
</li>

<style>
  .job {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3) var(--space-3);
    border-radius: var(--radius-md);
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .job:hover {
    background: var(--surface-hover);
  }

  .job.selected {
    background: var(--surface-selected);
  }

  .pick {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-3);
    align-items: center;
    min-block-size: var(--target-min-desktop);
    margin-inline: calc(-1 * var(--space-2));
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-sm);
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

  .platform {
    display: inline-flex;
    color: var(--text-secondary);
  }

  .target {
    font-family: var(--font-mono);
  }

  /* Room for two facts side by side; on a narrow window they drop under the target together. */
  .facts {
    display: inline-flex;
    flex: 1 1 calc(var(--space-16) * 3);
    flex-wrap: wrap;
    gap: var(--space-3);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .open {
    display: inline-flex;
    color: var(--text-tertiary);
  }

  .selected .open {
    color: var(--accent);
  }

  .note {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .failure {
    color: var(--status-down-text);
  }

  .actions {
    display: flex;
  }
</style>
