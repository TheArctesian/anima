<script lang="ts">
  import type { CardDevice } from '$lib/devices/api';
  import { press } from '$lib/motion/roles/press';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import { connectLabel, connectProtocols, webInterfacesOf } from './actions';
  import './actions.css';
  import WebMenu from './WebMenu.svelte';

  interface Props {
    device: CardDevice;
    /** The button is passed so the session tab can grow from it (WP-12 H-1, MOT-005). */
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
    /** Hidden while no route gives it (WP-26 D-3). */
    onOpenWeb?: (device: CardDevice, webInterface: WebInterface, button: HTMLElement) => void;
  }

  const { device, onConnect, onOpenWeb }: Props = $props();
  const web = $derived(webInterfacesOf(device));
</script>

<span class="actions">
  {#each connectProtocols(device) as protocol (protocol)}
    <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
    <button
      type="button"
      class="device-action"
      tabindex="0"
      aria-label={connectLabel(protocol, device.name)}
      onclick={(event) => {
        onConnect(device, protocol, event.currentTarget);
      }}
      {@attach press()}>{protocol}</button
    >
  {/each}
  {#if onOpenWeb !== undefined && web.length > 0}
    <WebMenu
      deviceName={device.name}
      interfaces={web}
      onOpen={(webInterface, button) => {
        onOpenWeb(device, webInterface, button);
      }}
    />
  {/if}
</span>

<style>
  .actions {
    flex: none;
    display: inline-flex;
    gap: var(--space-2);
  }
</style>
