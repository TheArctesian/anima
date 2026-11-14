<script lang="ts">
  import { resolve } from '$app/paths';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import RepositoryRow from './RepositoryRow.svelte';
  import ResourceRow from './ResourceRow.svelte';
  import type { ProjectsStore } from './store.svelte';
  import { primaryRepository, projectStatusWords } from './words';

  const { store, slug }: { store: ProjectsStore; slug: string } = $props();
  const project = $derived(store.current);
  const primary = $derived(project == null ? undefined : primaryRepository(project));
</script>

<section class="project">
  {#if store.status === 'loading'}
    <LoadingState label="Loading project" />
  {:else if store.status === 'failed'}
    <ErrorState message={`The project did not load. ${store.loadProblem}`} onRetry={() => void store.open(slug)} />
  {:else if project == null}
    <EmptyState title="Project not found" line="No project has this address." />
  {:else}
    <!-- One block for the page's identity: name, status word and the primary action. -->
    <header class="identity">
      <h1>{project.name}</h1>
      <span class="status">{projectStatusWords(project)}</span>
      {#if primary !== undefined}
        <a href={primary.url} target="_blank" rel="noreferrer">Open in GitHub</a>
      {/if}
    </header>
    <a class="back" href={resolve('/projects')}>All projects</a>
    {#if store.refusal !== null}
      <Notice tone="error" title="The hub refused" line={store.refusal.message} />
    {/if}
    <h2>Repositories</h2>
    <ul>
      {#each project.repositories as repository (repository.id)}
        <RepositoryRow {repository} onLeaveOut={(id) => void store.leaveOut(id)} onRestore={(id) => void store.restore(id)} />
      {/each}
    </ul>
    {#if project.resources.length > 0}
      <h2>Resources</h2>
      <ul>
        {#each project.resources as resource (resource.id)}
          <ResourceRow {resource} onConfirm={(id) => void store.confirm(id)} />
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .project {
    min-inline-size: 0;
  }

  .identity {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  h1 {
    margin: 0;
    font-size: var(--text-xl);
  }

  h2 {
    font-size: var(--text-lg);
  }

  .status {
    flex: 1;
    color: var(--text-secondary);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
