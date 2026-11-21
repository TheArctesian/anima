<script lang="ts">
  import type { Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import { listRow } from '$lib/motion/roles/transitions';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WakeApi } from '$lib/sessions/wake/api';
  import type { CardScreenSharing } from '$lib/devices/screen-sharing/setup';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import DeviceCard from './DeviceCard.svelte';
  import type { LiveCharges } from '$lib/metrics/battery/charges.svelte';
  import './geometry.css';
  import { reorderMotion } from '$lib/motion/roles/reorder.svelte';
  import { listItems, UNGROUPED } from '../list/look';
  import type { ListQuery } from '../query';
  import { HeldOrder } from '../list/held.svelte';

  interface Props {
    devices: readonly CardDevice[];
    beats: ReadonlyMap<string, readonly Beat[]>;
    /** The order and the status filter, which decide whether the tiles are grouped (D18). */
    query?: Pick<ListQuery, 'sort' | 'statuses'>;
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
    onOpenWeb?: (device: CardDevice, webInterface: WebInterface, button: HTMLElement) => void;
    screenSharing?: CardScreenSharing;
    wake?: WakeApi;
    /** The latest charge of each battery of each device (MET-032). */
    charges?: Pick<LiveCharges, 'of'>;
  }

  const { devices, beats, query = UNGROUPED, onConnect, onOpenWeb, wake, screenSharing, charges }: Props = $props();
  const held = new HeldOrder<CardDevice>(
    () => listItems(devices, query),
    () => `${query.sort} ${query.statuses.join(',')}`,
  );
  const items = $derived(held.items);
  const track = reorderMotion(() => items.map((item) => item.key).join('\n'));
</script>

<!-- A new card fades in after its stagger delay; a leaving card fades and keeps its place until it is gone (Reorder). -->
<ul class="grid" aria-label="Devices" {@attach held.hold}>
  {#each items as item, index (item.key)}
    <!-- The tile is the direct child of the loop, so its fade plays when the list changes, not only when its kind does. -->
    <li class:group={item.kind === 'group'} transition:listRow={{ index }} {@attach track(item.key)}>
      {#if item.kind === 'group'}
        {item.title}<span class="count">{item.count}</span>
      {:else}
        <DeviceCard device={item.device} beats={beats.get(item.device.id)} {onConnect} {onOpenWeb} {wake} {screenSharing} {charges} />
      {/if}
    </li>
  {/each}
</ul>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--device-card-min-inline-size)), 1fr));
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* A group title spans the grid: "Needs a look 4", then "Up 8" (D18). */
  .group {
    grid-column: 1 / -1;
    display: flex;
    align-items: end;
    min-block-size: var(--control-lg);
    padding: var(--space-4) var(--space-2) var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
  }

  .group:first-child {
    padding-block-start: 0;
  }

  .count {
    margin-inline-start: var(--space-2);
    color: var(--text-quiet);
    letter-spacing: 0;
  }
</style>
