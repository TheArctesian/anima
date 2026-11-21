<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { CallResult } from '../live/live.svelte';
  import { OVER_LIMIT } from '../live/run';

  interface Props {
    open: boolean;
    /** Raises the limit and sends the start that waited. */
    onRaise: () => Promise<CallResult>;
    onKeep: () => void;
  }

  const { open, onRaise, onKeep }: Props = $props();
  let busy = $state(false);
  let problem = $state<string | null>(null);

  async function raise(): Promise<void> {
    if (busy) return;
    busy = true;
    problem = null;
    const result = await onRaise();
    busy = false;
    if (!result.ok) problem = result.words;
  }
</script>

<!-- The soft limit (THR-114): the sentence of the operator, word for word. The person raises the limit, or the start
     does not go. -->
<Modal {open} title="Soft limit" onClose={onKeep} preset="sm" warning>
  <p>{OVER_LIMIT}</p>
  {#if problem !== null}<p class="problem" role="alert">{problem}</p>{/if}
  <div class="acts">
    <FormButton data-autofocus onclick={onKeep}>Keep the limit</FormButton>
    <FormButton tone="primary" onclick={() => void raise()} busy={busy}>Raise the limit</FormButton>
  </div>
</Modal>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .acts {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-block-start: var(--space-4);
  }

  .problem {
    color: var(--status-down);
    font-size: var(--text-sm);
  }
</style>
