<script lang="ts">
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { SessionsApi } from '../../api';
  import { WARNED, transition, type HostKeyEvent, type HostKeyStep } from './hostkey';

  interface Props {
    device: { readonly id: string; readonly name: string };
    api: Pick<SessionsApi, 'testConnection' | 'acceptHostKey'>;
    /** Close the warning; the old key stays accepted. */
    onclose: () => void;
    /** Start a new session; offered only after the new key was accepted. */
    onreconnect: () => void;
  }

  const { device, api, onclose, onreconnect }: Props = $props();
  let current: HostKeyStep = $state.raw(WARNED);
  let content: HTMLElement | undefined = $state();
  let closeButton: HTMLButtonElement | undefined = $state();
  const key = $derived('key' in current ? current.key : undefined);
  const problem = $derived(current.step === 'warned' ? current.problem : undefined);

  /** Move to the step `event` leads to; undefined (and no change) when it does not apply now. */
  function apply(event: HostKeyEvent): HostKeyStep | undefined {
    const next = transition(current, event);
    if (next !== undefined) current = next;
    return next;
  }

  async function review() {
    if (apply({ event: 'review' }) === undefined) return;
    const result = await api.testConnection({ deviceId: device.id });
    apply({ event: 'reviewed', result });
  }

  async function accept() {
    const sending = apply({ event: 'accept' });
    if (sending?.step !== 'accepting') return;
    // Exactly the fingerprint shown and confirmed; the state sends it once (SEC-007).
    const result = await api.acceptHostKey(device.id, sending.key.fingerprintSha256);
    apply({ event: 'accepted', result });
  }

  // "Close" is the default; when the focused button goes away with a step change, focus returns to it.
  $effect(() => {
    void current.step;
    if (content !== undefined && !content.contains(document.activeElement)) closeButton?.focus();
  });
</script>

<Modal open title="The host key of {device.name} has changed" warning describedBy="host-key-why" closeOnScrim={false} onClose={onclose}>
  <div class="content" bind:this={content}>
    <div id="host-key-why" class="why">
      <p>
        {device.name} answered with a different SSH host key from the one Anima accepted, so Anima did not connect.
      </p>
      <p>
        A new key is expected after the device is reinstalled or its SSH keys are made again. Otherwise it can mean
        that another machine is answering in its place, to read what you type, passwords included. Accept the new key
        only if you know why it changed. Until then, the old key stays.
      </p>
    </div>

    {#if key !== undefined}
      <dl class="key">
        <dt>New key, SHA-256 fingerprint ({key.algorithm})</dt>
        <dd class="fingerprint" data-fingerprint>{key.fingerprintSha256}</dd>
      </dl>
    {/if}
    {#if current.step === 'confirming'}
      <p class="confirm">Accept this fingerprint as the host key of {device.name}? Anima will trust it from now on.</p>
    {:else if current.step === 'accepted'}
      <p role="status">The new key is accepted. Reconnect to open the session.</p>
    {/if}
    {#if problem !== undefined}
      <p class="problem" role="status">{problem}</p>
    {/if}

    <div class="actions">
      <button type="button" bind:this={closeButton} onclick={onclose} {@attach press()}>Close</button>
      {#if current.step === 'warned'}
        <button type="button" onclick={review} {@attach press()}>Review new key</button>
      {:else if current.step === 'reviewing'}
        <button type="button" disabled>Reading the key…</button>
      {:else if current.step === 'reviewed'}
        <button type="button" class="warning" onclick={() => apply({ event: 'confirm' })} {@attach press()}>
          Accept new key
        </button>
      {:else if current.step === 'confirming'}
        <button type="button" onclick={() => apply({ event: 'back' })} {@attach press()}>Back</button>
        <button type="button" class="warning" onclick={accept} {@attach press()}>Yes, accept this key</button>
      {:else if current.step === 'accepting'}
        <button type="button" disabled>Accepting…</button>
      {:else}
        <button type="button" class="primary" onclick={onreconnect} {@attach press()}>Reconnect</button>
      {/if}
    </div>
  </div>
</Modal>

<style>
  /* The dialog's body spaces its children; the wrapper adds no box of its own. */
  .content {
    display: contents;
  }

  .why {
    display: grid;
    gap: var(--space-3);
  }

  p {
    margin: 0;
  }

  .key {
    margin: 0;
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--status-warning-tint);
  }

  dt {
    font-size: var(--text-sm);
  }

  .fingerprint {
    margin: var(--space-1) 0 0;
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }

  .confirm {
    font-weight: var(--weight-bold);
  }

  .problem {
    color: var(--text);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--status-down-tint);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  button {
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:disabled {
    color: var(--text-muted);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /*
   * Text on a --status-warning fill reaches only 4.39:1 with --bg (UI-002), so the warning is the border on the
   * tint: --text on --status-warning-tint is 7.5:1, and the border against --surface is 3.5:1.
   */
  .warning {
    border-color: var(--status-warning);
    background: var(--status-warning-tint);
    color: var(--text);
  }

  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
  }
</style>
