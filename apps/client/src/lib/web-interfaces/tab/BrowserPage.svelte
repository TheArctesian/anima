<script lang="ts">
  import { displayFingerprint, type Fingerprint } from '$lib/web-interfaces/shell/shell';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    label: string;
    url: string;
    /** The pinned fingerprint, or `null` while it is being read. */
    pin: Fingerprint | null;
    /** Whether an open is running: the certificate is read and compared first. */
    opening: boolean;
    onopen: () => void;
  }

  const { label, url, pin, opening, onopen }: Props = $props();
</script>

<!-- D-6, D-9: where a web view cannot pin, the page opens in the system browser beside the pinned fingerprint. -->
<section class="browser" tabindex="-1">
  <p>{label} ({url}) opens in your system browser: Anima cannot pin its certificate in a web view on this computer.</p>
  {#if pin === null}
    <p role="status">Reading the certificate…</p>
  {:else}
    <dl>
      <dt>Pinned certificate, SHA-256 fingerprint</dt>
      <dd class="fingerprint" data-fingerprint>{displayFingerprint(pin)}</dd>
    </dl>
    <p>Anima checks that the device still presents this certificate before it opens the page. The browser may ask you to trust it too: compare its fingerprint with this one.</p>
    <button type="button" disabled={opening} onclick={onopen} {@attach press()}>
      <Icon name="carbon:earth" />
      <span>{opening ? 'Checking the certificate…' : 'Open in system browser'}</span>
    </button>
  {/if}
</section>

<style>
  .browser {
    /* Room for the focus ring when the page takes focus. */
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    inline-size: min(100%, calc(var(--space-16) * 9));
    color: var(--text);
    font-size: var(--text-md);
    line-height: var(--leading-normal);
  }

  .browser:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  p,
  dl {
    margin: 0;
  }

  dt {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .fingerprint {
    margin: var(--space-1) 0 0;
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }

  button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--surface-base);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:active {
    background: var(--accent-pressed);
  }

  button:disabled {
    border-color: var(--border-strong);
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
