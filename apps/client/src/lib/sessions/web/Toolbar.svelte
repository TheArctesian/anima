<script lang="ts">
  import { displayFingerprint } from '$lib/desktop/webView/webView';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { TlsStatus } from './tab';

  interface Props {
    /** The page shown now, as read-only text. */
    url: string;
    tls: TlsStatus;
    /** Whether back and forward can be used: a web view is open in the tab. */
    navigable: boolean;
    reloadable: boolean;
    browsable: boolean;
    poppable: boolean;
    onback: () => void;
    onforward: () => void;
    onreload: () => void;
    onbrowser: () => void;
    onpopout: () => void;
  }

  const { url, tls, navigable, reloadable, browsable, poppable, onback, onforward, onreload, onbrowser, onpopout }: Props = $props();
  const pinned = $derived(tls.tls === 'pinned' ? displayFingerprint(tls.pin) : undefined);
</script>

<!-- WEB-005: back, forward, reload, the URL, the TLS status, "Open in system browser" and "Pop out". -->
<div class="toolbar" role="toolbar" aria-label="Web page">
  <button type="button" disabled={!navigable} onclick={onback} {@attach press()}>Back</button>
  <button type="button" disabled={!navigable} onclick={onforward} {@attach press()}>Forward</button>
  <button type="button" class="icon-only" aria-label="Reload" title="Reload" disabled={!reloadable} onclick={onreload} {@attach press()}>
    <Icon name="nf-md-refresh" />
  </button>
  <p class="url" data-url title={url}>{url}</p>
  <p class="tls" data-tls={tls.tls}>
    {#if tls.tls === 'plain'}
      <Icon name="nf-md-alert" size="sm" /><span>Not encrypted</span>
    {:else if tls.tls === 'trusted'}
      <Icon name="nf-md-lock" size="sm" /><span>Encrypted</span>
    {:else}
      <Icon name="nf-md-lock" size="sm" /><span>Pinned certificate</span>
      <span class="fingerprint" data-fingerprint title={pinned}>{pinned}</span>
    {/if}
  </p>
  <button type="button" disabled={!browsable} onclick={onbrowser} {@attach press()}>
    <Icon name="nf-md-web" size="sm" /><span>Open in system browser</span>
  </button>
  <button type="button" disabled={!poppable} onclick={onpopout} {@attach press()}>Pop out</button>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-thin) solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  p {
    margin: 0;
  }

  .url {
    flex: 1 1 auto;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
  }

  .tls {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-inline-size: 0;
    color: var(--text-muted);
  }

  .tls[data-tls='plain'] :global(.icon) {
    color: var(--status-warning);
  }

  .fingerprint {
    max-inline-size: calc(var(--space-16) * 2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
  }

  button {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-1);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .icon-only {
    justify-content: center;
    min-inline-size: var(--target-min-desktop);
    padding: 0;
  }

  button:disabled {
    color: var(--text-muted);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
