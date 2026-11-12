<script lang="ts">
  import { tick } from 'svelte';
  import Disclosure from '../../../forms/disclosure/Disclosure.svelte';
  import FormButton from '../../../forms/FormButton.svelte';
  import type { PolicyDraft } from './draft.svelte';
  import { LIMIT_SENTENCE } from './policy';

  interface Props {
    policy: PolicyDraft;
    open: boolean;
  }

  let { policy, open = $bindable() }: Props = $props();
  const unavailable = $derived(policy.phase !== 'ready');
  const retryShown = $derived(policy.phase === 'failed' || policy.retrying);
  let limitField = $state<HTMLInputElement>();

  /** Try again keeps focus while it runs; a successful load moves it to the limit field. */
  async function retry(): Promise<void> {
    if (policy.phase !== 'failed') return;
    if (!(await policy.load())) return;
    await tick();
    limitField?.focus();
  }
</script>

{#if policy.phase !== 'absent'}
  <div data-section="sessions">
    <Disclosure label="Sessions" level={3} bind:open>
      <div class="field">
        <label for="device-policy-sessionLimit">Session limit</label>
        <input
          id="device-policy-sessionLimit"
          inputmode="numeric"
          autocomplete="off"
          placeholder="No limit"
          disabled={unavailable}
          aria-invalid={policy.message === LIMIT_SENTENCE}
          bind:this={limitField}
          aria-describedby="device-policy-sessionLimit-hint device-policy-message"
          bind:value={policy.values.sessionLimit}
        />
        <small id="device-policy-sessionLimit-hint">The most live sessions at once, from 1 to 100. File tunnels do not count.</small>
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
        {#if retryShown}
          <FormButton aria-disabled={policy.phase !== 'failed'} aria-busy={policy.phase === 'loading'} onclick={() => void retry()}>Try again</FormButton>
        {/if}
      </div>
    </Disclosure>
  </div>
{/if}

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    color: var(--text-secondary);
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
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  input:not([type='checkbox']) {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background-color: var(--surface-raised);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
  }

  input:disabled {
    color: var(--text-secondary);
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
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    block-size: var(--target-min-desktop);
  }

  .message :global(button) {
    flex: none;
  }

  .message small {
    min-inline-size: 0;
    color: var(--status-down);
  }
</style>
