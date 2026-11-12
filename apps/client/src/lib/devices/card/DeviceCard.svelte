<script lang="ts">
  import { resolve } from '$app/paths';
  import HeartbeatBar from '$lib/checks/HeartbeatBar.svelte';
  import { uptimeText, type Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import DeviceActions from '$lib/devices/actions/DeviceActions.svelte';
  import WarningBadge from '$lib/devices/actions/WarningBadge.svelte';
  import MetricBars from '$lib/metrics/bars/MetricBars.svelte';
  import { press } from '$lib/motion/roles/press';
  import { sharedItem } from '$lib/motion/shared/shared';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WakeApi } from '$lib/sessions/wake/api';
  import type { CardScreenSharing } from '$lib/devices/screen-sharing/setup';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import OsIcon from '$lib/devices/os/OsIcon.svelte';
  import { hostLine } from './card';
  import './geometry.css';
  import StatusEdge from './StatusEdge.svelte';
  import StatusPill from './StatusPill.svelte';

  interface Props {
    device: CardDevice;
    /** The device's last heartbeats; `undefined` while they are not loaded, so no bar shows. */
    beats: readonly Beat[] | undefined;
    /** The card's protocol buttons; the button is passed so the session tab can grow from it (WP-12 H-2, RA-030). */
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
    /** The card's Web button (WEB-003); the button is hidden while no route gives this. */
    onOpenWeb?: (device: CardDevice, webInterface: WebInterface, button: HTMLElement) => void;
    screenSharing?: CardScreenSharing;
    /** The Wake button's call (RA-031); the button is hidden while no route gives this. */
    wake?: WakeApi;
  }

  const { device, beats, onConnect, onOpenWeb, wake, screenSharing }: Props = $props();
  const line = $derived(hostLine(device));
</script>

<article class="card">
  <StatusEdge status={device.status} />
  <header>
    <span class="identity">
      <OsIcon os={device.os} distribution={device.distribution} />
      <h2>
        <a
          href="{resolve('/devices')}/{encodeURIComponent(device.id)}"
          title={device.name}
          tabindex="0"
          data-device-id={device.id}
          {@attach press()}
          {@attach sharedItem(`device-name:${device.id}`)}
        >
          {device.name}
        </a>
      </h2>
    </span>
    <span class="marks">
      <WarningBadge warnings={device.accessWarnings} />
      <StatusPill status={device.status} deviceId={device.id} />
    </span>
  </header>
  <p class="host">
    <span title={line.host}>{line.host}</span>
    {#if line.ip !== null}<span class="ip">{line.ip}</span>{/if}
  </p>
  <p class="uptime"><span>Uptime, 24 h</span> <strong>{uptimeText(device.uptime)}</strong></p>
  <MetricBars readings={device.readings ?? null} source={device.metricsSource} />
  <footer>
    {#if beats !== undefined}<HeartbeatBar {beats} />{/if}
    <DeviceActions {device} {onConnect} {onOpenWeb} {wake} {screenSharing} />
  </footer>
</article>

<style>
  /* A fixed block size from tokens, the same as the loading skeleton's (MOT-007). */
  .card {
    position: relative;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: repeat(4, auto) 1fr;
    gap: var(--space-2);
    block-size: var(--device-card-block-size);
    padding: var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    color: var(--text);
    overflow: hidden;
  }

  header {
    min-inline-size: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  /* The OS logo before the name; only the name gives way, so the logo never moves the header (MOT-007). */
  .identity {
    min-inline-size: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .marks {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  h2 {
    margin: 0;
    min-inline-size: 0;
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  /* A block, so the press can scale it; a long name ends in an ellipsis. */
  a {
    display: block;
    min-block-size: var(--target-min-desktop);
    line-height: var(--target-min-desktop);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    text-decoration: none;
    transition: color var(--duration-fast) var(--ease-enter);
  }

  a:hover {
    color: var(--accent);
  }

  a:active {
    transition-duration: var(--duration-press);
    color: var(--text-secondary);
  }

  .host,
  .uptime {
    min-inline-size: 0;
    display: flex;
    gap: var(--space-2);
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
    overflow: hidden;
    white-space: nowrap;
  }

  .host span {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ip {
    flex: none;
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

  a:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
