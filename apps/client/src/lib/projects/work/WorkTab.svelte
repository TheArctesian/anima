<script lang="ts">
  import type { Project } from '../api';
  import RepositoryGate from '../forge/RepositoryGate.svelte';
  import type { ForgeApi } from '../forge/api';
  import Issues from './Issues.svelte';
  import PullRequests from './PullRequests.svelte';
  import Runs from './Runs.svelte';

  interface Props {
    project: Project;
    api: ForgeApi;
    now?: () => Date;
  }

  const { project, api, now = () => new Date() }: Props = $props();
</script>

<!-- The Work tab (PRJ-031, OD-E14): what is running and failing first, then the pull requests to merge, then the issues. -->
<RepositoryGate {project} {api}>
  {#snippet children(repository)}
    <Runs {api} repository={repository.id} defaultBranch={repository.defaultBranch ?? 'main'} {now} />
    <PullRequests {api} repository={repository.id} {now} />
    <Issues {api} repository={repository.id} {now} />
  {/snippet}
</RepositoryGate>
