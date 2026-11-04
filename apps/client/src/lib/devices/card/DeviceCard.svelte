<script lang="ts">
  import { resolve } from '$app/paths';
  import HeartbeatBar from '$lib/checks/HeartbeatBar.svelte';
  import { uptimeText, type Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import { press } from '$lib/motion/roles/press';
  import { hostLine } from './card';
  import './geometry.css';
  import StatusEdge from './StatusEdge.svelte';
  import StatusPill from './StatusPill.svelte';

  interface Props {
    device: CardDevice;
    /** The device's last heartbeats; `undefined` while they are not loaded, so no bar shows. */
    beats: readonly Beat[] | undefined;
    /** The card's "SSH" button; the button is passed so the session tab can grow from it (WP-12 H-2). */
    onConnect: (deviceId: string, button: HTMLElement) => void;
  }

  const { device, beats, onConnect }: Props = $props();
  const line = $derived(hostLine(device));
</script>

<article class="card">
  <StatusEdge status={device.status} />
  <header>
    <h2>
      <a href="{resolve('/devices')}/{encodeURIComponent(device.id)}" data-device-id={device.id} {@attach press()}>
        {device.name}
      </a>
    </h2>
    <StatusPill status={device.status} deviceId={device.id} />
  </header>
  <p class="host">
    <span>{line.host}</span>
    {#if line.ip !== null}<span class="ip">{line.ip}</span>{/if}
  </p>
  <p class="uptime"><span>Uptime, 24 h</span> <strong>{uptimeText(device.uptime)}</strong></p>
  <footer>
    {#if beats !== undefined}<HeartbeatBar {beats} />{/if}
    {#if device.ssh.enabled}
      <button
        type="button"
        aria-label="SSH to {device.name}"
        onclick={(event) => {
          onConnect(device.id, event.currentTarget);
        }}
        {@attach press()}>SSH</button
      >
    {/if}
  </footer>
</article>

<style>
  /* A fixed block size from tokens, the same as the loading skeleton's (MOT-007). */
  .card {
    position: relative;
    box-sizing: border-box;
    display: grid;
    grid-template-rows: repeat(3, auto) 1fr;
    gap: var(--space-2);
    block-size: var(--device-card-block-size);
    padding: var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  h2 {
    margin: 0;
    min-inline-size: 0;
    font-size: var(--text-md);
    line-height: var(--leading-tight);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  a {
    color: var(--text);
    text-decoration: none;
    transition: color var(--duration-fast) var(--ease-enter);
  }

  a:hover {
    color: var(--accent);
  }

  .host,
  .uptime {
    display: flex;
    gap: var(--space-2);
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
    overflow: hidden;
    white-space: nowrap;
  }

  .ip {
    font-family: var(--font-mono);
  }

  strong {
    color: var(--text);
    font-weight: var(--weight-regular);
  }

  footer {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: var(--space-2);
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--border);
  }

  a:focus-visible,
  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
