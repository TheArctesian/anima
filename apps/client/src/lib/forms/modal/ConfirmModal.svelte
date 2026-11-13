<script lang="ts">
  import type { HubError, HubResult } from '$lib/hub/hub';
  import FormButton from '../FormButton.svelte';
  import Modal from './Modal.svelte';

  interface Props {
    open: boolean;
    title: string;
    body: string;
    /** Short lines under the body, such as what the change lets a program do; none for most confirmations. */
    lines?: readonly string[];
    /** For example "Delete nas". */
    confirmLabel: string;
    /** Runs once; on success the buttons stay off, so the owner leaves or sets `open` to false. */
    onConfirm: () => Promise<HubResult<unknown>>;
    onCancel: () => void;
  }

  const { open, title, body, lines = [], confirmLabel, onConfirm, onCancel }: Props = $props();
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

<Modal {open} {title} onClose={onCancel} preset="sm">
  <p>{body}</p>
  {#if lines.length > 0}
    <ul>
      {#each lines as line (line)}<li>{line}</li>{/each}
    </ul>
  {/if}
  {#if failure !== null}
    <p class="failure" role="alert">{failure.message}</p>
  {/if}
  {#snippet footer()}
    <FormButton data-autofocus onclick={onCancel}>Cancel</FormButton>
    <!-- The label stays while busy, so the button keeps its width and Cancel does not move (MOT-007); aria-busy says it works. -->
    <FormButton tone="danger" onclick={confirm} aria-disabled={busy || done} aria-busy={busy}>
      {confirmLabel}
    </FormButton>
  {/snippet}
</Modal>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: var(--space-3) 0 0;
    padding-inline-start: var(--space-5);
    line-height: var(--leading-normal);
  }

  .failure {
    color: var(--status-down);
  }
</style>
