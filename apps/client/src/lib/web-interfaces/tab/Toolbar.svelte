<script lang="ts">
  import { displayFingerprint } from '$lib/web-interfaces/shell/shell';
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
    /** The Reload button, so focus can go to it after an accept. */
    reloadButton?: HTMLButtonElement;
  }

  let {
    url,
    tls,
    navigable,
    reloadable,
    browsable,
    poppable,
    onback,
    onforward,
    onreload,
    onbrowser,
    onpopout,
    reloadButton = $bindable(),
  }: Props = $props();

  const TLS_TEXT = {
    plain: 'Not encrypted',
    trusted: 'Encrypted',
    pinned: 'Pinned',
    untrusted: 'Not trusted',
    changed: 'Certificate changed',
    unknown: 'Not verified',
  } as const;

  const pinned = $derived(tls.tls === 'pinned' ? displayFingerprint(tls.pin) : undefined);
  const warned = $derived(tls.tls === 'plain' || tls.tls === 'untrusted' || tls.tls === 'changed');
  const title = $derived(pinned === undefined ? TLS_TEXT[tls.tls] : `Pinned certificate, SHA-256 ${pinned}`);
</script>

<!--
  WEB-005: back, forward, reload, the URL, the TLS status, "Open in system browser" and "Pop out". One line of a
  fixed height in every state and width, so the view area below never moves (MV-004); the URL takes the slack.
-->
<div class="toolbar" role="toolbar" aria-label="Web page">
  <button type="button" disabled={!navigable} onclick={onback} {@attach press()}>Back</button>
  <button type="button" disabled={!navigable} onclick={onforward} {@attach press()}>Forward</button>
  <button
    type="button"
    class="icon-only"
    aria-label="Reload"
    title="Reload"
    disabled={!reloadable}
    bind:this={reloadButton}
    onclick={onreload}
    {@attach press()}
  >
    <Icon name="nf-md-refresh" />
  </button>
  <p class="url" data-url title={url}>{url}</p>
  <p class="tls" data-tls={tls.tls} class:warned {title}>
    <Icon name={warned ? 'nf-md-alert' : 'nf-md-lock'} size="sm" />
    <span class="label">{TLS_TEXT[tls.tls]}</span>
    {#if pinned !== undefined}
      <span class="hidden" data-fingerprint>{pinned}</span>
    {/if}
  </p>
  <button type="button" aria-label="Open in system browser" disabled={!browsable} onclick={onbrowser} {@attach press()}>
    <Icon name="nf-md-web" size="sm" /><span class="label">Open in system browser</span>
  </button>
  <button type="button" disabled={!poppable} onclick={onpopout} {@attach press()}>Pop out</button>
</div>

<style>
  .toolbar {
    container-type: inline-size;
    display: flex;
    flex-wrap: nowrap;
    align-items: center;
    gap: var(--space-2);
    box-sizing: content-box;
    block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    overflow: hidden;
    border-bottom: var(--border-thin) solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
    white-space: nowrap;
  }

  p {
    margin: 0;
  }

  .url {
    flex: 1 1 auto;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
  }

  .tls {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-1);
    color: var(--text-muted);
  }

  .warned :global(.icon) {
    color: var(--status-warning);
  }

  /* Read by assistive technology, not drawn: the full fingerprint is in the title as well. */
  .hidden {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
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
    outline-offset: calc(var(--border-focus) * -1);
  }

  /* Narrow tabs keep the icons; the words stay for assistive technology and in the titles. */
  @container (max-inline-size: 720px) {
    .label {
      position: absolute;
      inline-size: 1px;
      block-size: 1px;
      overflow: hidden;
      clip-path: inset(50%);
    }
  }
</style>
