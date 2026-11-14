<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { ProjectResource } from './api';
  import { KIND_ICONS } from './tile';
  import { evidenceWords, resourceKindWords, resourceStateWords } from './words';

  interface Props {
    resource: ProjectResource;
    /** Without it the row only reports; the list shows resources, the detail page confirms them. */
    onConfirm?: (resourceId: string) => void;
  }

  const { resource, onConfirm }: Props = $props();
  const proposed = $derived(resource.state === 'FOUND_IN_REPOSITORY');
</script>

<!-- One place the project runs, a hairline row (UR-7, D9): its kind, and whether the operator has confirmed it. A
     proposal is marked in the warning colour and names its sign until it is confirmed (PRO-004). -->
<li class="resource" class:proposed>
  <span class="mark"><Icon name={KIND_ICONS[resource.kind]} size="sm" /></span>
  <span class="kind">{resourceKindWords(resource.kind)}</span>
  <span class="state">{resourceStateWords(resource)}</span>
  {#if proposed}
    <span class="signs">{resource.evidence.map(evidenceWords).join(', ')}</span>
    {#if onConfirm !== undefined}
      <FormButton size="sm" icon="carbon:checkmark" onclick={() => onConfirm(resource.id)}>Confirm</FormButton>
    {/if}
  {/if}
</li>

<style>
  .resource {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    min-block-size: var(--control-lg);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .mark {
    display: inline-flex;
    flex: none;
    color: var(--text-tertiary);
  }

  .kind {
    font-size: var(--text-sm);
  }

  .state {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  .proposed .state {
    color: var(--status-warning);
  }

  .signs {
    flex: 1;
    min-inline-size: 0;
    overflow-wrap: anywhere;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }
</style>
