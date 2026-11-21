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
  import { reorderMotion } from '$lib/motion/roles/reorder.svelte';
  import { listRow } from '$lib/motion/roles/transitions';
  import ProjectBrowse from './ProjectBrowse.svelte';
  import type { ProjectsStore } from './store.svelte';
  import { GROUP_ORDER, GROUP_TITLES, groupOf, type TileGroup } from './tile';
  import { SvelteSet } from 'svelte/reactivity';
  import GroupHead from './groups/GroupHead.svelte';
  import SelectBar from './groups/SelectBar.svelte';
  import { ProjectGroups } from './groups/groups.svelte';
  import { sectionsByGroup, storedClosed, storedShowBy, storeView, type ShowBy } from './groups/view';

  interface Props {
    store: ProjectsStore;
    /**
     * True for a hub that the application reaches at a port of this computer (a hosted hub through the link, or a hub
     * on this computer): GitHub cannot open such a hub in a browser, so only the key file of a GitHub App connects it.
     */
    keyFileOnly?: boolean;
    /** The operator's groups of cards, kept on this computer (PRO-059). */
    projectGroups?: ProjectGroups;
  }

  const { store, keyFileOnly = false, projectGroups = new ProjectGroups(localStorage) }: Props = $props();
  let showing = $state<'main' | 'leftOut'>('main');
  const projects = $derived(showing === 'main' ? store.main : store.leftOut);
  /** The one group the operator chose to see, or all of them (R-224). */
  let only = $state<'all' | TileGroup>('all');
  /** By the state of each project or by the operator's groups, kept on this computer (PRO-061). */
  let showBy = $state<ShowBy>(storedShowBy(localStorage));
  /** The group heads that are closed: a group ID, or "" for "Ungrouped". */
  const closed = new SvelteSet<string>(storedClosed(localStorage));
  $effect(() => storeView(localStorage, showBy, [...closed]));
  const byOwnGroup = $derived(showing === 'main' && showBy === 'group');
  const grouped = $derived(showing === 'main' && store.byName && !byOwnGroup);
  const byGroup = $derived(GROUP_ORDER.map((group) => ({ group, title: GROUP_TITLES[group], projects: projects.filter((project) => groupOf(project) === group) })));
  /** The main view in groups, problems first, or in the operator's groups; the left-out view is one plain group. */
  const groups = $derived.by(() => {
    if (byOwnGroup) return sectionsByGroup(projects, projectGroups.groups, (id) => projectGroups.of(id)).map((each) => ({ key: each.id ?? '', ...each }));
    if (grouped) return byGroup.filter((each) => each.projects.length > 0 && (only === 'all' || only === each.group)).map((each) => ({ key: each.group, id: null, ...each }));
    return [{ key: 'all', id: null, title: null, projects }];
  });
  const open = (key: string) => !byOwnGroup || !closed.has(key);
  /** A tile that is left out, restored or filtered fades where it stands; the others slide to their places (5.7.3, Reorder). */
  const track = reorderMotion(() =>
    groups
      .filter((group) => open(group.key))
      .flatMap((group) => group.projects.map((project) => project.id))
      .join('\n'),
  );
  /** Select mode (PRO-060): the check marks of the tiles and the bar of actions. */
  let selecting = $state(false);
  const selected = new SvelteSet<string>();
  const shownIds = $derived(groups.flatMap((group) => group.projects.map((project) => project.id)));

  function toggleSelecting(): void {
    selecting = !selecting;
    selected.clear();
  }

  function toggleHead(key: string): void {
    if (closed.has(key)) closed.delete(key);
    else closed.add(key);
  }
  /** Each choice says how many of the projects on the page it holds. */
  const choices = $derived([
    { value: 'all' as const, label: `All ${String(projects.length)}` },
    ...byGroup.map((each) => ({ value: each.group, label: `${each.title} ${String(each.projects.length)}` })),
  ]);
  const SHOW_BY = [
    { value: 'status', label: 'Status' },
    { value: 'group', label: 'Group' },
  ] as const;
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
      {#if showing === 'main' && store.byName}<Segmented label="Show by" size="sm" options={SHOW_BY} bind:value={showBy} />{/if}
      {#if grouped && projects.length > 0}<Segmented label="Which group" size="sm" options={choices} bind:value={only} />{/if}
      <ProjectBrowse {store} />
      {#if projects.length > 0}
        <span class="select"><FormButton size="sm" tone={selecting ? 'secondary' : 'ghost'} aria-pressed={selecting} onclick={toggleSelecting}>Select</FormButton></span>
      {/if}
    </div>
  {/if}
  {#if selecting}
    <SelectBar groups={projectGroups} {selected} shown={shownIds} onDone={toggleSelecting} />
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
    {#each groups as group (group.key)}
      <section class="group" aria-label={group.title ?? (showing === 'leftOut' ? 'Left out' : 'All projects')}>
        {#if byOwnGroup}
          <GroupHead groups={projectGroups} section={{ id: group.id, title: group.title ?? '', projects: group.projects }} open={open(group.key)} onToggle={() => toggleHead(group.key)} controls={`group-${group.key || 'none'}`} />
        {:else if group.title !== null}
          <h2>{group.title}<span class="count">{group.projects.length}</span></h2>
        {/if}
        {#if open(group.key)}
          <ul class="tiles" id={`group-${group.key || 'none'}`}>
            {#each group.projects as project, index (project.id)}
              <li class="entry" transition:listRow={{ index }} {@attach track(project.id)}>
                <ProjectTile
                  {project}
                  onLeaveOut={(id) => void store.leaveOut(id)}
                  onRestore={(id) => void store.restore(id)}
                  selected={selecting ? selected.has(project.id) : undefined}
                  onSelect={(on) => (on ? selected.add(project.id) : selected.delete(project.id))}
                  groupName={byOwnGroup ? undefined : projectGroups.of(project.id)?.name}
                />
              </li>
            {:else}
              <li class="none">No project in this group yet.</li>
            {/each}
          </ul>
        {/if}
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

  .select {
    margin-inline-start: auto;
  }

  .none {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
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

  /* The tile fills its cell, so the tiles of one row keep one height. */
  .entry {
    display: flex;
    min-inline-size: 0;
  }
</style>
