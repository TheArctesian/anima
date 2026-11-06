<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { fade, modal } from '$lib/motion/roles/transitions';
  import type { SessionsApi } from '../../api';
  import { trapFocus } from './focus';
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
  let surface: HTMLElement | undefined = $state();
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
    if (surface !== undefined && !surface.contains(document.activeElement)) closeButton?.focus();
  });
</script>

<div class="layer">
  <div class="scrim" transition:fade><div class="shade"></div></div>
  <div
    class="surface"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="host-key-title"
    aria-describedby="host-key-why"
    bind:this={surface}
    transition:modal
    {@attach trapFocus(onclose)}
  >
    <h2 id="host-key-title"><Icon name="nf-md-alert" /> The host key of {device.name} has changed</h2>
    <div id="host-key-why">
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
</div>

<style>
  /* Anchored at the top, so the surface grows downward when the key appears instead of re-centring (MV-004). */
  .layer {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: start center;
    z-index: var(--layer-modal);
  }

  .scrim,
  .surface {
    grid-area: 1 / 1;
  }

  .scrim {
    place-self: stretch;
  }

  .surface {
    margin-block-start: var(--space-16);
    max-block-size: calc(100vh - var(--space-16) * 2);
    overflow-y: auto;
  }

  /* The opacity sits on the child, so the scrim's own fade runs 0 to 1. */
  .shade {
    block-size: 100%;
    background: var(--bg);
    opacity: 0.72;
  }

  .surface {
    position: relative;
    box-sizing: border-box;
    inline-size: min(100% - var(--space-8), calc(var(--space-16) * 9));
    padding: var(--space-6);
    border: var(--border-thin) solid var(--border);
    border-top: var(--border-focus) solid var(--status-warning);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--elevation-3);
    color: var(--text);
    font-size: var(--text-md);
    line-height: var(--leading-normal);
  }

  h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-3);
    font-size: var(--text-lg);
    line-height: var(--leading-tight);
  }

  h2 :global(.icon) {
    color: var(--status-warning);
  }

  p {
    margin: 0 0 var(--space-3);
  }

  .key {
    margin: 0 0 var(--space-3);
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
    transition: background-color var(--duration-press) var(--ease-enter);
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
