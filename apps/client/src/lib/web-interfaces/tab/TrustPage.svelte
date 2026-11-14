<script lang="ts">
  import { displayFingerprint } from '$lib/web-interfaces/shell/shell';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { WebStep } from './tab';

  type TrustStep = Extract<WebStep, { step: 'untrusted' | 'accepting' | 'mismatch' }>;

  interface Props {
    current: TrustStep;
    /** Whether the stop for a changed certificate came first, so `accepting` replaces a pin. */
    changed: boolean;
    deviceName: string;
    label: string;
    url: string;
    onaccept: () => void;
    onconfirm: () => void;
    onback: () => void;
  }

  const { current, changed, deviceName, label, url, onaccept, onconfirm, onback }: Props = $props();
  const presented = $derived(displayFingerprint(current.presented));
  const expected = $derived(current.step === 'mismatch' ? displayFingerprint(current.expected) : undefined);
  const problem = $derived(current.step === 'accepting' ? undefined : current.problem);
  // Several web tabs can be mounted at once, so the ids are the component's own.
  const uid = $props.id();
</script>

<!-- WEB-008 and SEC-007: the fingerprint is shown in full before any accept; a changed one stops the page. -->
<section class="trust" aria-labelledby="{uid}-title" aria-describedby="{uid}-why" tabindex="-1" data-trust={changed ? 'changed' : 'first'}>
  {#if changed}
    <h2 id="{uid}-title"><Icon name="carbon:warning-alt" /> The certificate of {label} on {deviceName} has changed</h2>
    <div id="{uid}-why">
      <p>{label} ({url}) presented a different certificate from the one Anima pinned, so Anima stopped loading it.</p>
      <p>
        A new certificate is expected after the device is reset or its certificate is renewed. Otherwise it can mean
        that another machine is answering in its place, to read what you type, passwords included. Accept the new
        certificate only if you know why it changed. Until then, the pinned certificate stays.
      </p>
    </div>
  {:else}
    <h2 id="{uid}-title"><Icon name="carbon:locked" /> Check the certificate of {label} on {deviceName}</h2>
    <div id="{uid}-why">
      <p>{label} ({url}) presented a certificate that this computer does not trust, so Anima has not loaded it.</p>
      <p>
        Compare the fingerprint below with the one the device shows, on its screen or in its settings. Accept it only
        if they are the same. Anima then pins it for this web interface and stops if it ever changes.
      </p>
    </div>
  {/if}

  <dl class="fingerprints">
    {#if expected !== undefined}
      <dt>Pinned certificate, SHA-256 fingerprint</dt>
      <dd class="fingerprint" data-expected>{expected}</dd>
    {/if}
    <dt>{changed ? 'Presented certificate' : 'Certificate'}, SHA-256 fingerprint</dt>
    <dd class="fingerprint" data-presented>{presented}</dd>
  </dl>

  {#if current.step === 'mismatch' && current.confirming}
    <p class="confirm">Accept this fingerprint as the certificate of {label}? Anima will trust it from now on.</p>
  {/if}
  {#if problem !== undefined}
    <p class="problem" role="status">{problem}</p>
  {/if}

  <div class="actions">
    {#if current.step === 'accepting'}
      <button type="button" disabled>Accepting…</button>
    {:else if current.step === 'untrusted'}
      <button type="button" class="warning" onclick={onaccept} {@attach press()}>Accept certificate</button>
    {:else if current.confirming}
      <button type="button" onclick={onback} {@attach press()}>Back to the warning</button>
      <button type="button" class="warning" onclick={onaccept} {@attach press()}>Yes, accept this certificate</button>
    {:else}
      <button type="button" class="warning" onclick={onconfirm} {@attach press()}>Accept new certificate</button>
    {/if}
  </div>
</section>

<style>
  .trust {
    box-sizing: border-box;
    inline-size: min(100%, calc(var(--space-16) * 9));
    padding: var(--space-6);
    border: var(--border-thin) solid var(--border-strong);
    border-top: var(--border-focus) solid var(--status-warning);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-md);
    line-height: var(--leading-normal);
  }

  .trust:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
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

  .fingerprints {
    margin: 0 0 var(--space-3);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--status-warning-tint);
  }

  dt {
    font-size: var(--text-sm);
  }

  .fingerprint {
    margin: var(--space-1) 0 var(--space-2);
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }

  .confirm {
    font-weight: var(--weight-medium);
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
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:disabled {
    color: var(--text-secondary);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* --text on --status-warning-tint is 7.5:1 (UI-002), as the host key warning. */
  .warning {
    border-color: var(--status-warning);
    background: var(--status-warning-tint);
    color: var(--text);
  }
</style>
