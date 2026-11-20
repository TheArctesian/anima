<script lang="ts">
  import { resolve } from '$app/paths';
  import type { Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { press } from '$lib/motion/roles/press';
  import { reorderMotion } from '$lib/motion/roles/reorder.svelte';
  import { listRow } from '$lib/motion/roles/transitions';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import { connectLabel, connectProtocols } from '../actions/actions';
  import { hostLine, statusLabel } from '../card/card';
  import { listItems, reasonOf, UNGROUPED, type ListEntry } from '../list/look';
  import type { ListQuery } from '../query';
  import './list.css';

  interface Props {
    devices: readonly CardDevice[];
    beats: ReadonlyMap<string, readonly Beat[]>;
    /** The order and the status filter, which decide whether the rows are grouped (D18). */
    query?: Pick<ListQuery, 'sort' | 'statuses'>;
    /** The button is passed so the session tab can grow from it (MOT-005). */
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
  }

  const { devices, beats, query = UNGROUPED, onConnect }: Props = $props();
  const items: readonly ListEntry<CardDevice>[] = $derived(listItems(devices, query));
  const track = reorderMotion(() => items.map((item) => item.key).join('\n'));
  const ICONS: Record<SessionProtocol, IconName> = { SSH: 'carbon:terminal', RDP: 'carbon:remote-connection', VNC: 'carbon:screen' };
</script>

<!-- The device list in the phone layout (mockup 4): problems first under their own title. The row opens the device;
     its button opens a session with the first protocol that the device has. A long name ends with an ellipsis (MOB-015). -->
<div class="phone-devices" role="list" aria-label="Devices">
  {#each items as item, index (item.key)}
    <!-- One wrapper per entry, the loop's direct child: it moves to a new place and fades in and out (Reorder, 5.7.3). -->
    <div class="entry" role="none" transition:listRow={{ index }} {@attach track(item.key)}>
      {#if item.kind === 'group'}
        <h2 class="group">{item.title}<span class="count">{item.count}</span></h2>
      {:else}
        {@const device = item.device}
        {@const protocol = connectProtocols(device)[0]}
        {@const reason = device.status === 'UP' ? null : reasonOf(device, beats.get(device.id), Date.now())}
        <div class="dev" role="listitem" data-edge={device.status === 'UP' ? undefined : device.status.toLowerCase()}>
          <a class="what" href="{resolve('/devices')}/{encodeURIComponent(device.id)}" tabindex="0" data-device-id={device.id} {@attach press()}>
            <span class="name">{device.name}</span>
            <span class="line">
              {#if device.status === 'UP'}
                <span class="dot up" aria-hidden="true"></span>{statusLabel(device.status)}
              {:else}
                <span class="badge {device.status.toLowerCase()}">{statusLabel(device.status)}</span>
              {/if}
              <span class="host">{hostLine(device).host}</span>
            </span>
            {#if reason !== null}<span class="why">{reason.text}</span>{/if}
          </a>
          {#if protocol !== undefined}
            <button
              type="button"
              class="go"
              tabindex="0"
              aria-label={connectLabel(protocol, device.name)}
              onclick={(event) => {
                onConnect(device, protocol, event.currentTarget);
              }}
              {@attach press()}><Icon name={ICONS[protocol]} />{protocol}</button
            >
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</div>
