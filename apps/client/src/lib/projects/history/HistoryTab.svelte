<script lang="ts">
  import type { Project } from '../api';
  import RepositoryGate from '../forge/RepositoryGate.svelte';
  import type { ForgeApi } from '../forge/api';
  import Commits from './Commits.svelte';
  import Releases from './Releases.svelte';
  import Branches from './Branches.svelte';

  interface Props {
    project: Project;
    api: ForgeApi;
    now?: () => Date;
  }

  const { project, api, now = () => new Date() }: Props = $props();
</script>

<!-- The History tab (PRJ-031): the commits of a branch with their checks, then the releases and the branches. -->
<RepositoryGate {project} {api}>
  {#snippet children(repository)}
    <Commits {api} repository={repository.id} defaultBranch={repository.defaultBranch ?? 'main'} {now} />
    <Releases {api} repository={repository.id} {now} />
    <Branches {api} repository={repository.id} {now} />
  {/snippet}
</RepositoryGate>
