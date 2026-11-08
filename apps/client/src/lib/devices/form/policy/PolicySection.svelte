<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { PolicyDraft } from './draft.svelte';
  import { LIMIT_SENTENCE } from './policy';

  interface Props {
    policy: PolicyDraft;
  }

  const { policy }: Props = $props();
  const unavailable = $derived(policy.phase !== 'ready');
  const failed = $derived(policy.phase === 'failed');
</script>

<div class="field">
  <label for="device-policy-sessionLimit">Session limit</label>
  <input
    id="device-policy-sessionLimit"
    inputmode="numeric"
    autocomplete="off"
    placeholder="No limit"
    disabled={unavailable}
    aria-invalid={policy.message === LIMIT_SENTENCE}
    aria-describedby="device-policy-sessionLimit-hint device-policy-message"
    bind:value={policy.values.sessionLimit}
  />
  <small id="device-policy-sessionLimit-hint">The most live sessions at once, from 1 to 100. Viewers and file tunnels do not count.</small>
</div>
<div class="field">
  <label class="check">
    <input type="checkbox" disabled={unavailable} aria-describedby="device-policy-record-hint" bind:checked={policy.values.recordSessions} />
    Record sessions
  </label>
  <small id="device-policy-record-hint">
    Recordings are encrypted at rest. A "Recording" badge shows during a recorded session.
  </small>
</div>
<!-- The message line is always as tall as the Try again button, so loading, loaded and failed look alike. -->
<div class="message">
  <small id="device-policy-message" role="status">{policy.message}</small>
  {#if failed}
    <FormButton onclick={() => void policy.load()}>Try again</FormButton>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    color: var(--text);
  }

  small {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  input:not([type='checkbox']) {
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
