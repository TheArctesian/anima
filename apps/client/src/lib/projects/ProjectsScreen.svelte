<script lang="ts">
  import { resolve } from '$app/paths';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import RepositoryRow from './RepositoryRow.svelte';
  import ResourceRow from './ResourceRow.svelte';
  import type { ProjectsStore } from './store.svelte';

  const { store }: { store: ProjectsStore } = $props();
  let showing = $state<'main' | 'leftOut'>('main');
  const projects = $derived(showing === 'main' ? store.main : store.leftOut);
</script>

<section class="projects">
  <h1>Projects</h1>
  <div class="views" role="group" aria-label="Which projects">
    <FormButton size="sm" tone={showing === 'main' ? 'primary' : 'secondary'} aria-pressed={showing === 'main'} onclick={() => (showing = 'main')}>Projects</FormButton>
    <FormButton size="sm" tone={showing === 'leftOut' ? 'primary' : 'secondary'} aria-pressed={showing === 'leftOut'} onclick={() => (showing = 'leftOut')}>Left out</FormButton>
  </div>
  {#if store.refusal !== null}
    <Notice tone="error" title="The hub refused" line={store.refusal.message} />
  {/if}
  {#if store.status === 'loading'}
    <LoadingState label="Loading projects" />
  {:else if store.status === 'failed'}
    <ErrorState message={`Projects did not load. ${store.loadProblem}`} onRetry={() => void store.load()} />
  {:else if projects.length === 0 && showing === 'main'}
    <EmptyState title="No projects yet" line="Connect GitHub in Settings > Integrations">
      {#snippet action()}
        <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the section anchor is added -->
        <a href={`${resolve('/settings')}#settings-integrations`}>Settings > Integrations</a>
      {/snippet}
    </EmptyState>
  {:else if projects.length === 0}
    <EmptyState title="Nothing left out" line="A repository you leave out appears here, and Restore brings it back." />
  {:else}
    <ul class="entries">
      {#each projects as project (project.id)}
        <li class="entry">
          <a class="name" href={`${resolve('/projects')}/${encodeURIComponent(project.slug)}`}>{project.name}</a>
          <ul class="repositories">
            {#each project.repositories as repository (repository.id)}
              <RepositoryRow {repository} onLeaveOut={(id) => void store.leaveOut(id)} onRestore={(id) => void store.restore(id)} />
            {/each}
          </ul>
          {#if project.resources.length > 0}
            <ul class="resources">
              {#each project.resources as resource (resource.id)}
                <ResourceRow {resource} />
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ul>
    {#if store.hasMore(showing)}
      <FormButton onclick={() => void store.more(showing)}>Show more</FormButton>
    {/if}
  {/if}
</section>

<style>
  .projects {
    min-inline-size: 0;
  }

  .views {
    display: flex;
    gap: var(--space-2);
    margin-block-end: var(--space-3);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .entry {
    padding-block: var(--space-3);
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .name {
    font-size: var(--text-lg);
  }

  .repositories,
  .resources {
    padding-inline-start: var(--space-3);
  }
</style>
