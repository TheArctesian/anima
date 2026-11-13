<script lang="ts">
  import type { Builder } from '../../api';
  import type { JobView, ReleaseView } from '../api';
  import { shortCommit } from '../release';
  import JobRow from './JobRow.svelte';
  import { releaseSummary } from './progress';
  import StatePill from './StatePill.svelte';

  interface Props {
    release: ReleaseView;
    /** The newest release: drawn larger, the first thing the operator reads (BLD-080). */
    prominent?: boolean;
    /** An earlier attempt at the version, folded under the newest: quieter, and named by its number. */
    attempt?: number | undefined;
    builders: readonly Builder[] | null;
    now: Date;
    selectedId: string | null;
    cancelError: { readonly jobId: string; readonly message: string } | null;
    onselect: (job: JobView) => void;
    oncancel: (job: JobView) => void;
  }

  const { release, prominent = false, attempt, builders, now, selectedId, cancelError, onselect, oncancel }: Props = $props();
  const summary = $derived(releaseSummary(release.jobs));
  /** How the card and its jobs' controls name the release; attempts at one version need their number to differ. */
  const name = $derived(attempt === undefined ? release.version : `${release.version}, attempt ${String(attempt)}`);

  /** The date and time a release was started, in the operator's locale. */
  function started(iso: string): string {
    const at = new Date(iso);
    return Number.isNaN(at.getTime()) ? '—' : at.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
  }
</script>

<article class="card {summary.tone}" class:prominent class:earlier={attempt !== undefined} aria-label="Release {name}">
  <header>
    <h3>{release.version}{#if attempt !== undefined}<span class="attempt">attempt {attempt}</span>{/if}</h3>
    <StatePill word={summary.word} tone={summary.tone} />
    <span class="count">{summary.published}</span>
    <p class="meta">
      <code>{shortCommit(release.commit)}</code> by {release.createdBy},
      <time datetime={release.createdAt}>{started(release.createdAt)}</time>
    </p>
  </header>
  <ul class="jobs">
    {#each release.jobs as job (job.id)}
      <JobRow
        {job}
        version={name}
        {builders}
        {now}
        selected={job.id === selectedId}
        problem={cancelError?.jobId === job.id ? cancelError.message : undefined}
        onselect={() => {
          onselect(job);
        }}
        oncancel={() => {
          oncancel(job);
        }}
      />
    {/each}
  </ul>
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border-inline-start: var(--space-1) solid var(--edge, var(--border-subtle));
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .active {
    --edge: var(--accent);
  }

  .failed {
    --edge: var(--status-down);
  }

  .done {
    --edge: var(--status-up);
  }

  .waiting {
    --edge: var(--status-degraded);
  }

  /* An earlier attempt keeps its colours in its pills, not on its edge, so the newest release stays the loud one. */
  .earlier {
    --edge: var(--border-subtle);
  }

  .earlier h3 {
    color: var(--text-secondary);
  }

  .attempt {
    margin-inline-start: var(--space-2);
    font-family: var(--font-ui);
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
  }

  .prominent {
    gap: var(--space-3);
    padding: var(--space-4);
  }

  header {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    align-items: center;
    padding-inline: var(--space-3);
  }

  h3 {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-lg);
    line-height: var(--leading-tight);
  }

  .prominent h3 {
    font-size: var(--text-2xl);
    letter-spacing: var(--tracking-display);
  }

  .count {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .meta {
    flex-basis: 100%;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  code {
    font-family: var(--font-mono);
  }

  .jobs {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  @media (prefers-contrast: more) {
    .card {
      outline: var(--border-thin) solid var(--border-strong);
    }
  }
</style>
