<script lang="ts">
  import { resolve } from '$app/paths';
  import Icon from '$lib/icons/Icon.svelte';
  import type { Project } from './api';
  import RepositoryRow from './RepositoryRow.svelte';
  import { KIND_ICONS, lookReason } from './tile';
  import { projectStatusWords, resourceKindWords } from './words';

  interface Props {
    project: Project;
    onLeaveOut: (repositoryId: string) => void;
    onRestore: (repositoryId: string) => void;
  }

  const { project, onLeaveOut, onRestore }: Props = $props();
  const reason = $derived(lookReason(project));
</script>

<!-- One project as a tile (UR-7, PRO-001): its name and the state of its repository, why it needs a look, where it runs,
     then its repositories with Leave out. A tile that needs a look carries a 2 px warning edge (D18). -->
<li class="entry tile" class:look={reason !== null}>
  <div class="top">
    <span class="name">
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); the slug is the hub's and is encoded -->
      <a href={`${resolve('/projects')}/${encodeURIComponent(project.slug)}`}><Icon name="carbon:folder" size="sm" />{project.name}</a>
      {#if project.repositories[0] !== undefined}<span class="repo">{project.repositories[0].fullName}</span>{/if}
    </span>
    <span class="state">{projectStatusWords(project)}</span>
  </div>
  {#if reason !== null}<p class="why">{reason}</p>{/if}
  <ul class="places" aria-label="Where it runs">
    {#each project.resources as resource (resource.id)}
      <li class:proposed={resource.state === 'FOUND_IN_REPOSITORY'}>
        <Icon name={KIND_ICONS[resource.kind]} size="sm" />{resourceKindWords(resource.kind)}
      </li>
    {:else}
      <li class="nowhere">Not running anywhere</li>
    {/each}
  </ul>
  <ul class="repositories">
    {#each project.repositories as repository (repository.id)}
      <RepositoryRow {repository} {onLeaveOut} {onRestore} />
    {/each}
  </ul>
</li>

<style>
  .tile {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    padding: var(--space-4);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    transition: border-color var(--duration-fast) var(--ease-enter);
  }

  .tile:hover {
    border-color: var(--border-strong);
  }

  .look {
    box-shadow: inset var(--border-focus) 0 0 var(--status-warning);
  }

  .top {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .name {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  .name a {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-weight: var(--weight-medium);
    text-decoration: none;
  }

  .name a :global(svg) {
    color: var(--accent);
  }

  .name a:hover {
    text-decoration: underline;
    text-underline-offset: var(--space-1);
  }

  .name a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  .repo {
    overflow: hidden;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  .look .state {
    color: var(--status-warning);
  }

  .why {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .places {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .places li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .places :global(svg) {
    color: var(--text-tertiary);
  }

  .places .proposed {
    color: var(--status-warning);
  }

  .places .nowhere {
    color: var(--text-tertiary);
  }

  /* The repositories close the tile; their hairlines sit inside it. */
  .repositories {
    margin-block-start: auto;
  }
</style>
