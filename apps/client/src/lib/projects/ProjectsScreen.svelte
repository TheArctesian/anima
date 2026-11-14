<script lang="ts">
  import { resolve } from '$app/paths';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import RepositoryRow from './RepositoryRow.svelte';
  import ResourceRow from './ResourceRow.svelte';
  import type { ProjectsStore } from './store.svelte';

  const { store }: { store: ProjectsStore } = $props();
  let showing = $state<'main' | 'leftOut'>('main');
  const projects = $derived(showing === 'main' ? store.main : store.leftOut);
  const VIEWS = [
    { value: 'main', label: 'Projects' },
    { value: 'leftOut', label: 'Left out' },
  ] as const;
</script>

<!-- The Projects page in the website's language (UR-7, D9): the page head with the choice of view at its end, then each
     project as a name over hairline rows of its repositories and the places it runs. -->
<section class="projects">
  <PageHeader title="Projects">
    {#snippet actions()}
      <Segmented label="Which projects" size="sm" options={VIEWS} bind:value={showing} />
    {/snippet}
  </PageHeader>
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
          <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); the slug is the hub's and is encoded -->
          <a class="name" href={`${resolve('/projects')}/${encodeURIComponent(project.slug)}`}>
            <span class="mark"><Icon name="carbon:folder" size="sm" /></span>{project.name}<span class="go"><Icon name="carbon:chevron-right" size="sm" /></span>
          </a>
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
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    min-inline-size: 0;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  .name {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-block-size: var(--control-md);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-medium);
    text-decoration: none;
  }

  .mark {
    display: inline-flex;
    color: var(--accent);
  }

  .go {
    display: inline-flex;
    color: var(--text-tertiary);
    transition: translate var(--duration-fast) var(--ease-enter);
  }

  .name:hover .go {
    translate: var(--space-1) 0;
  }

  .name:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  /* The repositories and resources sit under the name, their hairlines closing with one under the last. */
  .repositories,
  .resources {
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .resources {
    margin-block-start: var(--space-3);
  }
</style>
