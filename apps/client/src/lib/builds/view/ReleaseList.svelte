<script lang="ts">
  import type { Builder } from '../api';
  import type { JobView, ReleaseView } from './api';
  import JobRow from './JobRow.svelte';
  import { shortCommit } from './release';

  interface Props {
    /** Already newest first. */
    releases: readonly ReleaseView[];
    builders: readonly Builder[] | null;
    now: Date;
    selectedId: string | null;
    cancelError: { readonly jobId: string; readonly message: string } | null;
    onselect: (job: JobView) => void;
    oncancel: (job: JobView) => void;
  }

  const { releases, builders, now, selectedId, cancelError, onselect, oncancel }: Props = $props();

  /** The date and time a release was started, in the operator's locale. */
  function started(iso: string): string {
    const at = new Date(iso);
    return Number.isNaN(at.getTime()) ? '—' : at.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
  }
</script>

<ul class="releases">
  {#each releases as release (release.id)}
    <li aria-label="Release {release.version}">
      <h3>{release.version}</h3>
      <p class="meta">
        <code>{shortCommit(release.commit)}</code> by {release.createdBy},
        <time datetime={release.createdAt}>{started(release.createdAt)}</time>
      </p>
      <ul class="jobs">
        {#each release.jobs as job (job.id)}
          <JobRow
            {job}
            version={release.version}
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
    </li>
  {/each}
</ul>

<style>
  .releases,
  .jobs {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .releases {
    gap: var(--space-4);
  }

  .releases > li {
    padding: var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  h3 {
    margin: 0;
    font-size: var(--text-lg);
  }

  .meta {
    margin: 0 0 var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  code {
    font-family: var(--font-mono);
  }
</style>
