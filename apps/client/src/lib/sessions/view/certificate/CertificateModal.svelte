<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { fade, modal } from '$lib/motion/roles/transitions';
  import type { SessionsApi } from '../../api';
  import { trapFocus } from '../hostkey/focus';
  import { PROBLEM, READING, transition, type CertificateEvent, type CertificateStep } from './certificate';

  interface Props {
    device: { readonly id: string; readonly name: string };
    api: Pick<SessionsApi, 'testConnection' | 'acceptServerCertificate'>;
    /** Close the warning; the old certificate stays accepted. */
    onclose: () => void;
    /** The hub now trusts the certificate shown; the caller reconnects. */
    onaccepted: () => void;
  }

  const { device, api, onclose, onaccepted }: Props = $props();
  /** The longest fingerprint (32 bytes as hex pairs) and one line: each row keeps this room whatever it shows. */
  const RESERVE_FINGERPRINT = `sha256:${Array.from({ length: 32 }, () => '00').join(':')}`;
  const RESERVE_LINE = '0';
  let current: CertificateStep = $state.raw(READING);
  let surface: HTMLElement | undefined = $state();
  let closeButton: HTMLButtonElement | undefined = $state();
  /** Set on Cancel or Escape: an answer that arrives later changes nothing and calls nothing. */
  let closed = false;
  const certificate = $derived('certificate' in current ? current.certificate : undefined);
  const notice = $derived(current.step === 'shown' || current.step === 'reading' ? current.notice : undefined);
  const problem = $derived(current.step === 'failed' ? current.problem : undefined);
  /** Accept keeps one label, so its width does not change; reading and sending show as busy. */
  const acceptBusy = $derived(current.step === 'reading' || current.step === 'accepting');

  /** Move to the step `event` leads to; undefined (and no change) when it does not apply now. */
  function apply(event: CertificateEvent): CertificateStep | undefined {
    const next = transition(current, event);
    if (next !== undefined) current = next;
    return next;
  }

  async function read() {
    if (current.step !== 'reading') return;
    const result = await api.testConnection({ deviceId: device.id, protocol: 'RDP' });
    if (!closed) apply({ event: 'readDone', result });
  }

  function retry() {
    if (apply({ event: 'read' }) !== undefined) void read();
  }

  async function accept() {
    const sending = apply({ event: 'accept' });
    if (sending?.step !== 'accepting') return;
    // Exactly the fingerprint shown; the step sends it once (SEC-007).
    const result = await api.acceptServerCertificate(device.id, sending.certificate.fingerprintSha256);
    if (closed) return;
    const next = apply({ event: 'accepted', result });
    if (next?.step === 'accepted') onaccepted();
    else if (next?.step === 'reading') void read();
  }

  function close() {
    closed = true;
    onclose();
  }

  onMount(() => void read());

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
    aria-labelledby="certificate-title"
    aria-describedby="certificate-why"
    bind:this={surface}
    transition:modal
    {@attach trapFocus(close)}
  >
    <h2 id="certificate-title"><Icon name="nf-md-alert" /> The certificate of {device.name} has changed</h2>
    <div id="certificate-why">
      <p>
        {device.name} presented a different RDP server certificate from the one Anima accepted, so Anima did not
        connect.
      </p>
      <p>
        A new certificate is expected after the device is reinstalled or its certificate is renewed. Otherwise it can
        mean that another machine is answering in its place, to see your screen and read what you type, passwords
        included. Accept the new certificate only if you know why it changed. Until then, the old one stays.
      </p>
    </div>

    <!-- Each row holds a hidden copy of the longest value it can show, so the buttons below do not move when the certificate arrives (MOT-007, MV-004). -->
    <dl class="certificate" aria-busy={certificate === undefined}>
      <dt>New certificate, SHA-256 fingerprint</dt>
      <dd class="fingerprint">
        <span class="reserve" aria-hidden="true" data-reserve>{RESERVE_FINGERPRINT}</span>
        {#if certificate !== undefined}
          <span data-fingerprint>{certificate.fingerprintSha256}</span>
        {:else if current.step === 'reading'}
          <span class="pending">Reading the certificate…</span>
        {/if}
      </dd>
      <dt>Subject</dt>
      <dd>
        <span class="reserve" aria-hidden="true" data-reserve>{RESERVE_LINE}</span>
        {#if certificate?.subject === null}
          <span>Not given</span>
        {:else if certificate !== undefined}
          <span data-subject>{certificate.subject}</span>
        {/if}
      </dd>
      <dt>Valid until</dt>
      <dd>
        <span class="reserve" aria-hidden="true" data-reserve>{RESERVE_LINE}</span>
        {#if certificate?.notAfter === null}
          <span>Not given</span>
        {:else if certificate !== undefined}
          <span><time datetime={certificate.notAfter} data-not-after>{certificate.notAfter}</time></span>
        {/if}
      </dd>
    </dl>
    {#if current.step === 'unchanged'}
      <p class="notice" role="status">{PROBLEM.unchanged}</p>
    {/if}
    {#if current.step === 'accepted'}
      <p role="status">The new certificate is accepted. Reconnecting…</p>
    {/if}
    {#if notice !== undefined}
      <p class="notice" role="status">{notice}</p>
    {/if}
    {#if problem !== undefined}
      <p class="problem" role="status">{problem}</p>
    {/if}

    <div class="actions">
      <button type="button" bind:this={closeButton} onclick={close} {@attach press()}>
        {current.step === 'accepted' ? 'Close' : 'Cancel'}
      </button>
      {#if current.step === 'unchanged'}
        <button type="button" onclick={onaccepted} {@attach press()}>Reconnect</button>
      {:else if current.step === 'failed'}
        <button type="button" onclick={retry} {@attach press()}>Read again</button>
      {:else if current.step !== 'accepted'}
        <button
          type="button"
          class="warning"
          disabled={current.step !== 'shown'}
          aria-busy={acceptBusy}
          onclick={accept}
          {@attach press()}>Accept new certificate</button
        >
      {/if}
    </div>
  </div>
</div>

<style>
  /* Anchored at the top, so the surface grows downward when the certificate appears (MV-004). */
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

  .shade {
    block-size: 100%;
    background: var(--bg);
    opacity: 0.72;
  }

  .surface {
    position: relative;
    box-sizing: border-box;
    inline-size: min(100% - var(--space-8), calc(var(--space-16) * 9));
    max-block-size: calc(100vh - var(--space-16) * 2);
    margin-block-start: var(--space-16);
    overflow-y: auto;
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

  .certificate {
    margin: 0 0 var(--space-3);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--status-warning-tint);
  }

  dt {
    font-size: var(--text-sm);
  }

  /* The reserve and the value share one cell: the row is as tall as the taller of the two. */
  dd {
    display: grid;
    margin: var(--space-1) 0 var(--space-2);
    overflow-wrap: anywhere;
  }

  dd > * {
    grid-area: 1 / 1;
  }

  /* Takes its room, unseen and unread. */
  .reserve {
    visibility: hidden;
  }

  .pending {
    font-family: var(--font-ui);
    color: var(--text-muted);
  }

  .fingerprint {
    font-family: var(--font-mono);
  }

  .notice {
    font-weight: var(--weight-bold);
  }

  .problem {
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

  /* Text on a --status-warning fill is below 4.5:1 (UI-002); the warning is the border on the tint. */
  .warning {
    border-color: var(--status-warning);
    background: var(--status-warning-tint);
    color: var(--text);
  }
</style>
