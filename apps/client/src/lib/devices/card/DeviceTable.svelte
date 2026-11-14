<script lang="ts">
  import './geometry.css';
  import '../list/rows.css';
  import { resolve } from '$app/paths';
  import HeartbeatBar from '$lib/checks/HeartbeatBar.svelte';
  import { uptimeText, type Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import DeviceActions from '$lib/devices/actions/DeviceActions.svelte';
  import WarningBadge from '$lib/devices/actions/WarningBadge.svelte';
  import { press } from '$lib/motion/roles/press';
  import { listRow } from '$lib/motion/roles/transitions';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { CardScreenSharing } from '$lib/devices/screen-sharing/setup';
  import type { WakeApi } from '$lib/sessions/wake/api';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import OsIcon from '$lib/devices/os/OsIcon.svelte';
  import { reorderMotion } from '$lib/motion/roles/reorder.svelte';
  import { hostLine } from './card';
  import Meters from '../list/Meters.svelte';
  import Why from '../list/Why.svelte';
  import { listItems, UNGROUPED, type ListEntry } from '../list/look';
  import type { ListQuery } from '../query';
  import StatusBadge from '../status/StatusBadge.svelte';

  interface Props {
    devices: readonly CardDevice[];
    beats: ReadonlyMap<string, readonly Beat[]>;
    /** The order and the status filter, which decide whether the rows are grouped (D18). */
    query?: Pick<ListQuery, 'sort' | 'statuses'>;
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
    onOpenWeb?: (device: CardDevice, webInterface: WebInterface, button: HTMLElement) => void;
    wake?: WakeApi;
    screenSharing?: CardScreenSharing;
  }

  const { devices, beats, query = UNGROUPED, onConnect, onOpenWeb, wake, screenSharing }: Props = $props();
  const items: readonly ListEntry<CardDevice>[] = $derived(listItems(devices, query));
  const track = reorderMotion(() => items.map((item) => item.key).join('\n'));
</script>

<!-- The list view (UR-7, D18): hairline rows, problems first under their own group title, each problem with its
     reason where an up device shows its meters. Below 1100 px each device takes two lines. -->
<table class="device-rows">
  <caption class="caption">Devices</caption>
  <thead>
    <tr>
      <th scope="col" class="c-status">Status</th>
      <th scope="col" class="c-name">Name</th>
      <th scope="col" class="c-uptime">24 h</th>
      <th scope="col" class="c-beats">Last 50 checks</th>
      <th scope="col" class="c-detail">Usage, or what is wrong</th>
      <th scope="col" class="c-connect">Connect</th>
    </tr>
  </thead>
  <tbody>
    {#each items as item, index (item.key)}
      {#if item.kind === 'group'}
        <tr class="group" transition:listRow={{ index }} {@attach track(item.key)}>
          <th colspan="6" scope="colgroup">{item.title}<span class="count">{item.count}</span></th>
        </tr>
      {:else}
        {@const device = item.device}
        {@const line = hostLine(device)}
        {@const deviceBeats = beats.get(device.id)}
        <tr class="device" data-edge={device.status === 'UP' ? undefined : device.status.toLowerCase()} transition:listRow={{ index }} {@attach track(item.key)}>
          <td class="c-status">
            <span class="marks">
              <StatusBadge status={device.status} deviceId={device.id} />
              <WarningBadge warnings={device.accessWarnings} />
            </span>
          </td>
          <th scope="row" class="c-name">
            <OsIcon os={device.os} distribution={device.distribution} />
            <a href="{resolve('/devices')}/{encodeURIComponent(device.id)}" title={device.name} tabindex="0" data-device-id={device.id} {@attach press()}>{device.name}</a>
            <span class="host" title={line.host}>{line.host}</span>
            {#if line.ip !== null}<span class="host ip">{line.ip}</span>{/if}
          </th>
          <td class="c-uptime">{uptimeText(device.uptime)}</td>
          <td class="c-beats">{#if deviceBeats !== undefined}<HeartbeatBar beats={deviceBeats} />{/if}</td>
          <td class="c-detail">
            {#if device.status === 'UP'}
              <Meters readings={device.readings ?? null} source={device.metricsSource} />
            {:else}
              <Why {device} beats={deviceBeats} />
            {/if}
          </td>
          <td class="c-connect"><DeviceActions {device} {onConnect} {onOpenWeb} {wake} {screenSharing} compact /></td>
          <td class="c-meta" aria-hidden="true"><span class="host">{line.host}</span><span>{uptimeText(device.uptime)}</span></td>
        </tr>
      {/if}
    {/each}
  </tbody>
</table>
