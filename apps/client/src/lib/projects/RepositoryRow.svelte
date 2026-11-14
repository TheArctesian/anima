<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { ProjectRepository } from './api';
  import { repositoryStateWords } from './words';

  interface Props {
    repository: ProjectRepository;
    onLeaveOut: (repositoryId: string) => void;
    onRestore: (repositoryId: string) => void;
  }

  const { repository, onLeaveOut, onRestore }: Props = $props();
</script>

<li class="repository">
  <span class="name">{repository.fullName}</span>
  <span class="state">{repositoryStateWords(repository)}</span>
  {#if repository.leftOut}
    <FormButton size="sm" onclick={() => onRestore(repository.id)}>Restore</FormButton>
  {:else}
    <FormButton size="sm" tone="ghost" onclick={() => onLeaveOut(repository.id)}>Leave out</FormButton>
  {/if}
</li>

<style>
  .repository {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-block-size: var(--control-md);
  }

  .name {
    flex: 1;
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .state {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
