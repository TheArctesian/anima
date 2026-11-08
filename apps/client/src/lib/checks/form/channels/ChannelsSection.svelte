<script lang="ts">
  import { tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { KIND_LABELS } from './channels';
  import type { ChannelsDraft } from './draft.svelte';

  interface Props {
    draft: ChannelsDraft;
    /** The ID of the resend input, so the form can focus it after a refusal. */
    resendId: string;
  }

  const { draft, resendId }: Props = $props();
  const unavailable = $derived(draft.phase !== 'ready');
  const busy = $derived(draft.phase === 'loading');

  let root = $state<HTMLElement>();

  /** Reloads; focus stays on Try again while it works, and moves to the first control once the list is back. */
  async function retry(): Promise<void> {
    if (busy) return;
    const hadFocus = root?.contains(document.activeElement) === true;
    await draft.load();
    if (draft.phase !== 'ready' || !hadFocus) return;
    await tick();
    root?.querySelector<HTMLElement>('input:not(:disabled)')?.focus();
  }

  function toggle(id: string, on: boolean): void {
    draft.values.channelIds = on ? [...draft.values.channelIds, id] : draft.values.channelIds.filter((chosen) => chosen !== id);
  }
</script>

<fieldset data-section="notifications" bind:this={root}>
  <legend>Notifications</legend>
  <!-- The list box has one height in every state (MOT-007); a long list scrolls inside it. -->
  <div class="list" aria-busy={busy}>
    {#if draft.phase === 'loading'}
      {#each [0, 1, 2] as row (row)}
        <div class="row skeleton" aria-hidden="true"><span></span></div>
      {/each}
    {:else if draft.phase === 'ready'}
      {#each draft.channels as channel (channel.id)}
        <label class="row">
          <input
            type="checkbox"
            checked={draft.values.channelIds.includes(channel.id)}
            onchange={(event) => { toggle(channel.id, event.currentTarget.checked); }}
          />
          <span class="name">{channel.name}</span>
          <small>{KIND_LABELS[channel.kind]}</small>
        </label>
      {:else}
        <p class="none">See Settings → Notifications.</p>
      {/each}
    {/if}
  </div>
  <div class="field">
    <label for={resendId}>Resend every (Down beats)</label>
    <input
      id={resendId}
      inputmode="numeric"
      autocomplete="off"
      disabled={unavailable}
      aria-invalid={draft.resendInvalid}
      aria-describedby="{resendId}-hint {resendId}-message"
      bind:value={draft.values.resend}
    />
    <small id="{resendId}-hint">0 means never resend.</small>
  </div>
  <!-- The message line is as tall as the Try again button, so every state looks alike. -->
  <div class="message">
    <small id="{resendId}-message" role="status">{draft.message}</small>
    {#if draft.retryShown}
      <FormButton aria-disabled={busy ? 'true' : undefined} onclick={() => void retry()}>Try again</FormButton>
    {/if}
  </div>
</fieldset>

<style>
  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    padding: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .list {
    box-sizing: border-box;
    block-size: calc(var(--target-min-desktop) * 3 + var(--border-thin) * 2);
    overflow-y: auto;
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    box-sizing: border-box;
    block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    color: var(--text);
  }

  .name {
    flex: 1;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .skeleton span {
    flex: 1;
    block-size: var(--space-2);
    background: var(--surface-raised);
    border-radius: var(--radius-md);
  }

  .none {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .field label {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .field input {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background-color: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
  }

  input:disabled {
    color: var(--text-muted);
  }

  input[aria-invalid='true'] {
    border-color: var(--status-down);
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  small {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .message {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
  }

  .message small {
    color: var(--status-down);
  }
</style>
