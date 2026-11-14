<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import DetailBar from '$lib/page/DetailBar.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import Tabs from '$lib/tabs/Tabs.svelte';
  import { tabIds } from '$lib/tabs/tabs';
  import type { ProjectsStore } from '../store.svelte';
  import { lookReason } from '../tile';
  import { primaryRepository, projectStatusWords } from '../words';
  import Overview from './Overview.svelte';
  import { BUILT_TABS, PROJECT_TABS, readTab, type ProjectTab } from './tabs';

  interface Props {
    store: ProjectsStore;
    slug: string;
    /** The `?tab=` word of the page; an unbuilt or unknown tab is Overview. */
    tab?: string | null;
    /** Moves to another tab; the route keeps it in the address so Back and a reload return to it. */
    onTab?: (tab: ProjectTab) => void;
  }

  const { store, slug, tab = null, onTab }: Props = $props();
  const project = $derived(store.current);
  const primary = $derived(project == null ? undefined : primaryRepository(project));
  const found = $derived(store.status !== 'loading' && store.status !== 'failed' && project != null);
  const reason = $derived(project == null ? null : lookReason(project));
  const selected = $derived(readTab(tab));
  const items = $derived(PROJECT_TABS.filter((each) => BUILT_TABS.includes(each.id)));
</script>

{#snippet meta()}
  {#if project != null}
    <span class="status" class:look={reason !== null}>{projectStatusWords(project)}</span>
    {#if primary !== undefined}<span class="mono">{primary.fullName}</span>{/if}
  {/if}
{/snippet}

{#snippet actions()}
  {#if primary !== undefined}
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub, not a route of this app -->
    <a class="out" href={primary.url} target="_blank" rel="noreferrer"><Icon name="simple-icons:github" size="sm" />Open in GitHub</a>
  {/if}
{/snippet}

{#snippet why()}
  <span class="mark"><Icon name="carbon:warning-alt" size="sm" /></span><span>{reason}</span>
{/snippet}

{#snippet projectTabs()}
  <Tabs {items} {selected} label="Project" idPrefix="project" onSelect={(id) => onTab?.(readTab(id))} />
{/snippet}

<!-- A project's page (UR-7, D16, PRJ-030): one detail bar (the name, the state of its repository, the repository, Open in
     GitHub, and why it needs a look), then its tabs. A tab shows only once the hub serves it. -->
<section class="project">
  <DetailBar
    back={{ to: '/projects', label: 'Projects' }}
    icon="carbon:folder"
    title={found && project != null ? project.name : null}
    meta={found ? meta : undefined}
    actions={found ? actions : undefined}
    reason={found && reason !== null ? why : undefined}
    tabs={found && items.length > 1 ? projectTabs : undefined}
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
    <div id={tabIds('project', selected).panel} role={items.length > 1 ? 'tabpanel' : undefined} aria-labelledby={items.length > 1 ? tabIds('project', selected).tab : undefined}>
      {#if selected === 'overview'}
        <Overview
          {project}
          onLeaveOut={(id) => void store.leaveOut(id)}
          onRestore={(id) => void store.restore(id)}
          onConfirm={(id) => void store.confirm(id)}
        />
      {/if}
    </div>
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

  .status.look {
    color: var(--status-warning);
  }

  .mono {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .mark {
    display: inline-flex;
    color: var(--status-warning);
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
</style>
