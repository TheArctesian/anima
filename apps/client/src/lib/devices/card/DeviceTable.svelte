<script lang="ts">
  import './geometry.css';
  import { resolve } from '$app/paths';
  import HeartbeatBar from '$lib/checks/HeartbeatBar.svelte';
  import { uptimeText, type Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import { press } from '$lib/motion/roles/press';
  import { listRow } from '$lib/motion/roles/transitions';
  import { hostLine, offersSsh } from './card';
  import { reorderMotion } from './reorder.svelte';
  import StatusEdge from './StatusEdge.svelte';
  import StatusPill from './StatusPill.svelte';

  interface Props {
    devices: readonly CardDevice[];
    beats: ReadonlyMap<string, readonly Beat[]>;
    onConnect: (device: CardDevice, button: HTMLElement) => void;
  }

  const { devices, beats, onConnect }: Props = $props();
  const track = reorderMotion(() => devices.map((device) => device.id).join('\n'));
</script>

<!-- A wide table scrolls inside its own box and does not widen the page. -->
<div class="scroll">
<table>
  <caption class="caption">Devices</caption>
  <thead>
    <tr>
      <th scope="col">Name</th>
      <th scope="col">Status</th>
      <th scope="col">Host</th>
      <th scope="col">Uptime, 24 h</th>
      <th scope="col">Heartbeats</th>
      <th scope="col">SSH</th>
    </tr>
  </thead>
  <tbody>
    {#each devices as device, index (device.id)}
      {@const line = hostLine(device)}
      {@const deviceBeats = beats.get(device.id)}
      <tr transition:listRow={{ index }} {@attach track(device.id)}>
        <th scope="row">
          <a
            href="{resolve('/devices')}/{encodeURIComponent(device.id)}"
            title={device.name}
            tabindex="0"
            data-device-id={device.id}
            {@attach press()}
          >
            {device.name}
          </a>
        </th>
        <td class="status">
          <StatusEdge status={device.status} />
          <StatusPill status={device.status} deviceId={device.id} />
        </td>
        <td>
          <span class="host" title={line.host}>{line.host}</span>
          {#if line.ip !== null}<span class="ip">{line.ip}</span>{/if}
        </td>
        <td>{uptimeText(device.uptime)}</td>
        <td>{#if deviceBeats !== undefined}<HeartbeatBar beats={deviceBeats} />{/if}</td>
        <td>
          {#if offersSsh(device)}
            <button
              type="button"
              tabindex="0"
              aria-label="SSH to {device.name}"
              onclick={(event) => {
                onConnect(device, event.currentTarget);
              }}
              {@attach press()}>SSH</button
            >
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
</div>

<style>
  .caption {
    position: absolute;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .scroll {
    min-inline-size: 0;
    overflow-x: auto;
  }

  table {
    inline-size: 100%;
    border-collapse: collapse;
    color: var(--text);
  }

  th,
  td {
    box-sizing: border-box;
    block-size: var(--device-row-block-size);
    padding: 0 var(--space-3);
    border-bottom: var(--border-thin) solid var(--border);
    text-align: start;
    font-weight: var(--weight-regular);
    white-space: nowrap;
  }

  thead th {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  tbody tr {
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  tbody tr:hover {
    background: var(--surface-raised);
  }

  .status {
    position: relative;
  }

  .ip {
    margin-inline-start: var(--space-2);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  a {
    display: inline-block;
    min-block-size: var(--target-min-desktop);
    line-height: var(--target-min-desktop);
    color: var(--text);
    text-decoration: none;
  }

  a:active {
    color: var(--text-muted);
  }

  a:hover {
    color: var(--accent);
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

  button:active {
    background: var(--accent-tint);
  }

  a:focus-visible,
  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
