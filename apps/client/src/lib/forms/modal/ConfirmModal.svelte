<script lang="ts">
  import type { HubError, HubResult } from '$lib/hub/error';
  import FormButton from '../FormButton.svelte';
  import Modal from './Modal.svelte';

  interface Props {
    open: boolean;
    title: string;
    body: string;
    /** For example "Delete nas". */
    confirmLabel: string;
    /** Runs once; on success the buttons stay off, so the owner leaves or sets `open` to false. */
    onConfirm: () => Promise<HubResult<unknown>>;
    onCancel: () => void;
  }

  const { open, title, body, confirmLabel, onConfirm, onCancel }: Props = $props();
  let busy = $state(false);
  let done = $state(false);
  let failure = $state<HubError | null>(null);

  async function confirm(): Promise<void> {
    if (busy || done) return;
    busy = true;
    failure = null;
    const result = await onConfirm();
    busy = false;
    if (result.ok) done = true;
    else failure = result.error;
  }
</script>

<Modal {open} {title} onClose={onCancel}>
  <p>{body}</p>
  {#if failure !== null}
    <p class="failure" role="alert">{failure.message}</p>
  {/if}
  <div class="actions">
    <FormButton data-autofocus onclick={onCancel} disabled={done}>Cancel</FormButton>
    <FormButton tone="danger" onclick={confirm} disabled={busy || done} aria-busy={busy}>
      {busy ? 'Working' : confirmLabel}
    </FormButton>
  </div>
</Modal>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .failure {
    color: var(--status-down);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
