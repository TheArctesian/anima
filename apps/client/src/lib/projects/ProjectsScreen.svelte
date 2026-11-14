<script lang="ts">
  import { resolve } from '$app/paths';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import SmallCaps from '$lib/page/SmallCaps.svelte';
  import ProjectTile from './ProjectTile.svelte';
  import type { ProjectsStore } from './store.svelte';
  import { GROUP_ORDER, GROUP_TITLES, groupOf } from './tile';

  const { store }: { store: ProjectsStore } = $props();
  let showing = $state<'main' | 'leftOut'>('main');
  const projects = $derived(showing === 'main' ? store.main : store.leftOut);
  /** The main view in groups, problems first; the left-out view is one plain group. */
  const groups = $derived(
    showing === 'leftOut'
      ? [{ title: null, projects }]
      : GROUP_ORDER.map((group) => ({ title: GROUP_TITLES[group], projects: projects.filter((project) => groupOf(project) === group) })).filter(
          (group) => group.projects.length > 0,
        ),
  );
  const VIEWS = [
    { value: 'main', label: 'Projects' },
    { value: 'leftOut', label: 'Left out' },
  ] as const;
</script>

<!-- The Projects page in the website's language (UR-7): the page head with the choice of view at its end, then the
     projects as tiles in groups, problems first. -->
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
    {#each groups as group (group.title)}
      <section class="group" aria-label={group.title ?? 'Left out'}>
        {#if group.title !== null}<h2><SmallCaps>{group.title}</SmallCaps><span class="count">{group.projects.length}</span></h2>{/if}
        <ul class="tiles">
          {#each group.projects as project (project.id)}
            <ProjectTile {project} onLeaveOut={(id) => void store.leaveOut(id)} onRestore={(id) => void store.restore(id)} />
          {/each}
        </ul>
      </section>
    {/each}
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

  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font: inherit;
  }

  .count {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(var(--space-16) * 4.75), 1fr));
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
