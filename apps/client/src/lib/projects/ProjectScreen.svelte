<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import DetailBar from '$lib/page/DetailBar.svelte';
  import Section from '$lib/page/Section.svelte';
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
  const found = $derived(store.status !== 'loading' && store.status !== 'failed' && project != null);
</script>

{#snippet meta()}
  {#if project != null}<span class="status">{projectStatusWords(project)}</span>{/if}
{/snippet}

{#snippet actions()}
  {#if primary !== undefined}
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub, not a route of this app -->
    <a class="out" href={primary.url} target="_blank" rel="noreferrer"><Icon name="simple-icons:github" size="sm" />Open in GitHub</a>
  {/if}
{/snippet}

<!-- A project's page (UR-7, D16): one detail bar for its name, status and the way out to GitHub, then its repositories
     and the places it runs as sections of hairline rows. -->
<section class="project">
  <DetailBar
    back={{ to: '/projects', label: 'Projects' }}
    icon="carbon:folder"
    title={found && project != null ? project.name : null}
    meta={found ? meta : undefined}
    actions={found ? actions : undefined}
  />
  {#if store.status === 'loading'}
    <LoadingState label="Loading project" />
  {:else if store.status === 'failed'}
    <ErrorState message={`The project did not load. ${store.loadProblem}`} onRetry={() => void store.open(slug)} />
  {:else if project == null}
    <EmptyState title="Project not found" line="No project has this address." />
  {:else}
    {#if store.refusal !== null}
      <Notice tone="error" title="The hub refused" line={store.refusal.message} />
    {/if}
    <Section title="Repositories" icon="simple-icons:github">
      <ul>
        {#each project.repositories as repository (repository.id)}
          <RepositoryRow {repository} onLeaveOut={(id) => void store.leaveOut(id)} onRestore={(id) => void store.restore(id)} />
        {/each}
      </ul>
    </Section>
    {#if project.resources.length > 0}
      <Section title="Resources" icon="carbon:cloud-upload">
        <ul>
          {#each project.resources as resource (resource.id)}
            <ResourceRow {resource} onConfirm={(id) => void store.confirm(id)} />
          {/each}
        </ul>
      </Section>
    {/if}
  {/if}
</section>

<style>
  .project {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    min-inline-size: 0;
  }

  .status {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .out {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--control-md);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: var(--text-sm);
    text-decoration: none;
    white-space: nowrap;
  }

  .out:hover {
    background: var(--surface-hover);
  }

  .out:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }
</style>
