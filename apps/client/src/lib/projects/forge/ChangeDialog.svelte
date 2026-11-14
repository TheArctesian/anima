<script lang="ts">
  import type { Snippet } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { HubError, HubResult } from '$lib/hub/hub';

  interface Props {
    open: boolean;
    title: string;
    /** What the change does, in one sentence. */
    body: string;
    /** Facts the operator checks before saying yes, such as the commit it runs on (PRJ-034). */
    lines?: readonly string[];
    confirmLabel: string;
    tone?: 'primary' | 'danger';
    /** False while a field of the dialog is not filled as the change needs. */
    ready?: boolean;
    /** Sends the change once. On success the dialog calls `onDone`; on failure it shows the hub's fixed words and stays. */
    onConfirm: () => Promise<HubResult<unknown>>;
    onDone: () => void;
    onCancel: () => void;
    /** Fields of the change, such as the merge method. */
    children?: Snippet;
  }

  const { open, title, body, lines = [], confirmLabel, tone = 'primary', ready = true, onConfirm, onDone, onCancel, children }: Props =
    $props();
  let busy = $state(false);
  let failure = $state.raw<HubError | null>(null);

  async function confirm(): Promise<void> {
    if (busy || !ready) return;
    busy = true;
    failure = null;
    const result = await onConfirm();
    busy = false;
    if (result.ok) onDone();
    else failure = result.error;
  }
</script>

<!-- A change on GitHub (PRJ-028, OD-E14): what it does, the facts it acts on, then a yes. Nothing is sent before the yes. -->
<Modal {open} {title} onClose={onCancel} preset="sm">
  <p>{body}</p>
  {#if lines.length > 0}
    <ul>
      {#each lines as line (line)}<li>{line}</li>{/each}
    </ul>
  {/if}
  {#if children}<div class="fields">{@render children()}</div>{/if}
  {#if failure !== null}<p class="failure" role="alert">{failure.message}</p>{/if}
  {#snippet footer()}
    <FormButton data-autofocus onclick={onCancel}>Cancel</FormButton>
    <FormButton {tone} onclick={confirm} aria-disabled={busy || !ready} aria-busy={busy}>{confirmLabel}</FormButton>
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
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    overflow-wrap: anywhere;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .failure {
    color: var(--status-down-text);
  }
</style>
