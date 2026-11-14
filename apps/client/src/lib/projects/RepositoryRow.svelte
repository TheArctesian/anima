<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { ProjectRepository } from './api';
  import { repositoryStateWords } from './words';

  interface Props {
    repository: ProjectRepository;
    onLeaveOut: (repositoryId: string) => void;
    onRestore: (repositoryId: string) => void;
  }

  const { repository, onLeaveOut, onRestore }: Props = $props();
</script>

<!-- One repository, a hairline row (UR-7, D9): its GitHub name in mono, its state, and Leave out or Restore. -->
<li class="repository">
  <span class="mark"><Icon name="simple-icons:github" size="sm" /></span>
  <span class="name">{repository.fullName}</span>
  <span class="state">{repositoryStateWords(repository)}</span>
  {#if repository.leftOut}
    <FormButton size="sm" icon="carbon:renew" onclick={() => onRestore(repository.id)}>Restore</FormButton>
  {:else}
    <FormButton size="sm" tone="ghost" onclick={() => onLeaveOut(repository.id)}>Leave out</FormButton>
  {/if}
</li>

<style>
  .repository {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-block-size: var(--control-lg);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .mark {
    display: inline-flex;
    flex: none;
    color: var(--text-tertiary);
  }

  .name {
    flex: 1;
    min-inline-size: 0;
    overflow-wrap: anywhere;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .state {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
    white-space: nowrap;
  }
</style>
