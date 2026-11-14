<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { ProjectResource } from './api';
  import { evidenceWords, resourceKindWords, resourceStateWords } from './words';

  interface Props {
    resource: ProjectResource;
    /** Without it the row only reports; the list shows resources, the detail page confirms them. */
    onConfirm?: (resourceId: string) => void;
  }

  const { resource, onConfirm }: Props = $props();
  const proposed = $derived(resource.state === 'FOUND_IN_REPOSITORY');
</script>

<li class="resource">
  <span class="kind">{resourceKindWords(resource.kind)}</span>
  <span class="state">{resourceStateWords(resource)}</span>
  {#if proposed}
    <!-- The sign that produced the proposal, until the operator confirms it (PRO-004). -->
    <span class="signs">{resource.evidence.map(evidenceWords).join(', ')}</span>
    {#if onConfirm !== undefined}
      <FormButton size="sm" onclick={() => onConfirm(resource.id)}>Confirm</FormButton>
    {/if}
  {/if}
</li>

<style>
  .resource {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-3);
    min-block-size: var(--control-md);
  }

  .state,
  .signs {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .signs {
    flex: 1;
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }
</style>
