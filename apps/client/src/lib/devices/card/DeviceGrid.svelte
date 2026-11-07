<script lang="ts">
  import type { Beat } from '$lib/checks/heartbeats';
  import type { CardDevice } from '$lib/devices/api';
  import { listRow } from '$lib/motion/roles/transitions';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import DeviceCard from './DeviceCard.svelte';
  import './geometry.css';
  import { reorderMotion } from './reorder.svelte';

  interface Props {
    devices: readonly CardDevice[];
    beats: ReadonlyMap<string, readonly Beat[]>;
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
    onOpenWeb?: (device: CardDevice, webInterface: WebInterface, button: HTMLElement) => void;
  }

  const { devices, beats, onConnect, onOpenWeb }: Props = $props();
  const track = reorderMotion(() => devices.map((device) => device.id).join('\n'));
</script>

<!-- A new card fades in after its stagger delay; a leaving card fades and keeps its place until it is gone (Reorder). -->
<ul class="grid" aria-label="Devices">
  {#each devices as device, index (device.id)}
    <li transition:listRow={{ index }} {@attach track(device.id)}>
      <DeviceCard {device} beats={beats.get(device.id)} {onConnect} {onOpenWeb} />
    </li>
  {/each}
</ul>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--device-card-min-inline-size)), 1fr));
    gap: var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
