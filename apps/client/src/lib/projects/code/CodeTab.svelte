<script lang="ts">
  import type { Project } from '../api';
  import RepositoryGate from '../forge/RepositoryGate.svelte';
  import type { ForgeApi } from '../forge/api';
  import CodeBrowser from './CodeBrowser.svelte';

  interface Props {
    project: Project;
    api: ForgeApi;
    now?: () => Date;
  }

  const { project, api, now = () => new Date() }: Props = $props();
</script>

<!-- The Code tab (PRJ-031): the repository's files on a branch, read through the hub, one folder or file at a time. -->
<RepositoryGate {project} {api}>
  {#snippet children(repository)}
    <CodeBrowser {api} repository={repository.id} defaultBranch={repository.defaultBranch ?? 'main'} {now} />
  {/snippet}
</RepositoryGate>
