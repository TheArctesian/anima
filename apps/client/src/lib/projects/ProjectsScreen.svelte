<script lang="ts">
  import { resolve } from '$app/paths';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import ProjectTile from './ProjectTile.svelte';
  import ProjectBrowse from './ProjectBrowse.svelte';
  import type { ProjectsStore } from './store.svelte';
  import { GROUP_ORDER, GROUP_TITLES, groupOf, type TileGroup } from './tile';

  interface Props {
    store: ProjectsStore;
    /**
     * True for a hub that the application reaches at a port of this computer (a hosted hub through the link, or a hub
     * on this computer): GitHub cannot open such a hub in a browser, so only the key file of a GitHub App connects it.
     */
    keyFileOnly?: boolean;
  }

  const { store, keyFileOnly = false }: Props = $props();
  let showing = $state<'main' | 'leftOut'>('main');
  const projects = $derived(showing === 'main' ? store.main : store.leftOut);
  /** The one group the operator chose to see, or all of them (R-224). */
  let only = $state<'all' | TileGroup>('all');
  const grouped = $derived(showing === 'main' && store.byName);
  const byGroup = $derived(GROUP_ORDER.map((group) => ({ group, title: GROUP_TITLES[group], projects: projects.filter((project) => groupOf(project) === group) })));
  /** The main view in groups, problems first; the left-out view is one plain group. */
  const groups = $derived(
    grouped
      ? byGroup.filter((each) => each.projects.length > 0 && (only === 'all' || only === each.group))
      : [{ title: null, projects }],
  );
  /** Each choice says how many of the projects on the page it holds. */
  const choices = $derived([
    { value: 'all' as const, label: `All ${String(projects.length)}` },
    ...byGroup.map((each) => ({ value: each.group, label: `${each.title} ${String(each.projects.length)}` })),
  ]);
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
  <!-- With no project and no filter there is nothing to sort or to filter, so the controls are not drawn. -->
  {#if projects.length > 0 || store.filtered || !store.byName}
    <div class="tools">
      {#if grouped && projects.length > 0}<Segmented label="Which group" size="sm" options={choices} bind:value={only} />{/if}
      <ProjectBrowse {store} />
    </div>
  {/if}
  {#if store.refusal !== null}
    <Notice tone="error" title="The hub refused" line={store.refusal.message} />
  {/if}
  {#if store.status === 'loading'}
    <LoadingState label="Loading projects" />
  {:else if store.status === 'failed'}
    <ErrorState message={`Projects did not load. ${store.loadProblem}`} onRetry={() => void store.load()} />
  {:else if projects.length === 0 && store.filtered}
    <EmptyState title="No matching projects" line="Change the language or size filter." />
  {:else if projects.length === 0 && showing === 'main'}
    <EmptyState title="No projects yet" line={keyFileOnly ? 'Connect GitHub with the key file of a GitHub App.' : 'Connect GitHub.'}>
      {#snippet action()}
        <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the section anchor is added -->
        <a href={`${resolve('/settings')}#settings-integrations`}>Settings > Integrations</a>
      {/snippet}
    </EmptyState>
  {:else if projects.length === 0}
    <EmptyState title="Nothing left out" line="Repositories you leave out appear here." />
  {:else if groups.length === 0}
    <EmptyState title="No project in this group" line="Choose All to see each project." />
  {:else}
    {#each groups as group (group.title)}
      <section class="group" aria-label={group.title ?? (showing === 'leftOut' ? 'Left out' : 'All projects')}>
        {#if group.title !== null}<h2>{group.title}<span class="count">{group.projects.length}</span></h2>{/if}
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

  /* The choice of the group, then the order and the filters of the hub, on one line when they fit (R-224). */
  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3) var(--space-6);
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
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
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
