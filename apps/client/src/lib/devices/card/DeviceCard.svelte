<script lang="ts">
  import { resolve } from '$app/paths';
  import HeartbeatBar from '$lib/checks/HeartbeatBar.svelte';
  import { uptimeText, type Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import DeviceActions from '$lib/devices/actions/DeviceActions.svelte';
  import WarningBadge from '$lib/devices/actions/WarningBadge.svelte';
  import Meters from '../list/Meters.svelte';
  import Why from '../list/Why.svelte';
  import { press } from '$lib/motion/roles/press';
  import { sharedItem } from '$lib/motion/shared/shared';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WakeApi } from '$lib/sessions/wake/api';
  import type { CardScreenSharing } from '$lib/devices/screen-sharing/setup';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import OsIcon from '$lib/devices/os/OsIcon.svelte';
  import { hostLine } from './card';
  import './geometry.css';
  import StatusEdge from '../status/StatusEdge.svelte';
  import StatusBadge from '../status/StatusBadge.svelte';

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

<!-- A hairline tile (UR-7, D18): the name and its status, the host, then the reason a device needs a look and its
     meters, its last checks and uptime, and the ways to connect. -->
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
      <StatusBadge status={device.status} deviceId={device.id} />
    </span>
  </header>
  <p class="host"><span title={line.host}>{line.host}</span>{#if line.ip !== null}<span class="ip">{line.ip}</span>{/if}</p>
  <div class="middle">
    {#if device.status !== 'UP'}<Why {device} {beats} wrap />{/if}
    {#if device.status === 'UP' || device.readings != null}<Meters readings={device.readings ?? null} source={device.metricsSource} />{/if}
  </div>
  <p class="uptime">
    {#if beats !== undefined}<HeartbeatBar {beats} />{/if}
    <span><span class="label">Uptime, 24 h</span> {uptimeText(device.uptime)}</span>
  </p>
  <footer><DeviceActions {device} {onConnect} {onOpenWeb} {wake} {screenSharing} compact /></footer>
</article>

<style>
  /* A fixed block size from tokens, the same as the loading skeleton's (MOT-007). */
  .card {
    position: relative;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto auto 1fr auto auto;
    gap: var(--space-2);
    block-size: var(--device-card-block-size);
    padding: var(--space-4);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    color: var(--text);
    overflow: hidden;
    transition: border-color var(--duration-fast) var(--ease-enter);
  }

  .card:hover {
    border-color: var(--border-strong);
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
    color: var(--text-tertiary);
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
    font-weight: var(--weight-medium);
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
  }

  a:hover {
    text-decoration: underline;
    text-underline-offset: var(--space-1);
  }

  a:active {
    transition-duration: var(--duration-press);
    color: var(--text-secondary);
  }

  a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
    border-radius: var(--radius-sm);
  }

  .host {
    display: flex;
    gap: var(--space-2);
    margin: calc(var(--space-2) * -1) 0 0;
    overflow: hidden;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .host span {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ip {
    flex: none;
    color: var(--text-quiet);
  }

  .middle {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-2);
    min-block-size: 0;
    overflow: hidden;
  }

  .uptime {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .uptime > :global(:first-child) {
    flex: 1 1 auto;
  }

  .label {
    position: absolute;
    overflow: hidden;
    clip-path: inset(50%);
  }

  footer {
    display: flex;
    margin: 0 calc(var(--space-2) * -1) calc(var(--space-2) * -1);
  }
</style>
