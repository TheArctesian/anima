<script lang="ts">
  import { untrack, type Snippet } from 'svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import type { HubError } from '$lib/hub/hub';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import type { Project, ProjectRepository } from '../api';
  import type { ForgeApi } from './api';
  import type { RepositoryAccess } from './types';
  import { accessWords } from './words';

  interface Props {
    project: Project;
    api: ForgeApi;
    /** The tab's content for one repository that the hub can read now. */
    children: Snippet<[repository: ProjectRepository]>;
  }

  const { project, api, children }: Props = $props();
  const repositories = $derived(project.repositories.filter((each) => !each.leftOut && !each.gone));
  let chosen = $state<string | null>(null);
  const repository = $derived(repositories.find((each) => each.id === chosen) ?? repositories[0]);
  let access = $state.raw<ReadonlyMap<string, RepositoryAccess> | null>(null);
  let failure = $state.raw<HubError | null>(null);

  async function load(): Promise<void> {
    failure = null;
    const result = await api.access(project.slug);
    if (result.ok) access = result.value;
    else failure = result.error;
  }

  $effect(() => {
    void project.slug;
    untrack(() => void load());
  });

  const blocked = $derived(repository === undefined || access === null ? null : accessWords(access.get(repository.id) ?? 'NOT_CONNECTED'));
</script>

<!-- A project's lists are read for one repository at a time, and only when the hub can read it now (PRJ-032): the access
     comes first, so "not connected" shows before any list is asked for. -->
<div class="gate">
  {#if repositories.length > 1}
    <Segmented
      label="Repository"
      size="sm"
      options={repositories.map((each) => ({ value: each.id, label: each.fullName }))}
      value={repository?.id ?? ''}
      onchange={(value: string) => {
        chosen = value;
      }}
    />
  {/if}
  {#if repository === undefined}
    <EmptyState title="No repository" line="Every repository of this project is left out or gone." />
  {:else if failure !== null}
    <ErrorState message={failure.message} onRetry={() => void load()} />
  {:else if access === null}
    <LoadingState label="Loading" rows={3} />
  {:else if blocked !== null}
    <EmptyState title={blocked.title} line={blocked.line} />
  {:else}
    {#key repository.id}{@render children(repository)}{/key}
  {/if}
</div>

<style>
  .gate {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }
</style>
