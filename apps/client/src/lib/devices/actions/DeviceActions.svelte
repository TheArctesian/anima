<script lang="ts">
  import type { CardDevice } from '$lib/devices/api';
  import { press } from '$lib/motion/roles/press';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WakeApi } from '$lib/sessions/wake/api';
  import WakeButton from '$lib/sessions/wake/WakeButton.svelte';
  import ScreenSharingButton from '$lib/devices/screen-sharing/ScreenSharingButton.svelte';
  import { offersScreenSharing, type CardScreenSharing } from '$lib/devices/screen-sharing/setup';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import { canWake, connectLabel, connectProtocols, webInterfacesOf } from './actions';
  import './actions.css';
  import WebMenu from './WebMenu.svelte';

  interface Props {
    device: CardDevice;
    /** The button is passed so the session tab can grow from it (WP-12 H-1, MOT-005). */
    onConnect: (device: CardDevice, protocol: SessionProtocol, button: HTMLElement) => void;
    /** Hidden while no route gives it (WP-26 D-3). */
    onOpenWeb?: (device: CardDevice, webInterface: WebInterface, button: HTMLElement) => void;
    /** The wake call (RA-031); the Wake button is hidden while no route gives it. */
    wake?: WakeApi;
    /** The "Screen sharing" button for a macOS or Linux device with VNC off (R-079); hidden while no route gives it. */
    screenSharing?: CardScreenSharing;
  }

  const { device, onConnect, onOpenWeb, wake, screenSharing }: Props = $props();
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
  {#if screenSharing !== undefined && offersScreenSharing(device)}
    <ScreenSharingButton {device} calls={screenSharing.calls} onReady={screenSharing.onReady} compact />
  {/if}
  {#if onOpenWeb !== undefined && web.length > 0}
    <WebMenu
      deviceName={device.name}
      interfaces={web}
      onOpen={(webInterface, button) => {
        onOpenWeb(device, webInterface, button);
      }}
    />
  {/if}
  {#if wake !== undefined && canWake(device)}
    <!-- Up needs no wake. The slot is one button tall, and the button's status line floats above it: it moves nothing (MOT-007). -->
    <span class="wake-slot"><WakeButton deviceId={device.id} api={wake} disabled={device.status === 'UP'} /></span>
  {/if}
</span>

<style>
  .actions {
    flex: none;
    display: inline-flex;
    gap: var(--space-2);
  }

  .wake-slot {
    position: relative;
    flex: none;
    block-size: var(--target-min-desktop);
  }

  /* WakeButton draws its two-line status under itself; here it lies over the card above the button, out of the layout. */
  .wake-slot :global(.status) {
    position: absolute;
    inset-block-end: 100%;
    inset-inline-end: 0;
    z-index: var(--layer-popover);
    inline-size: max-content;
    max-inline-size: calc(var(--space-16) * 4);
    min-block-size: 0;
    margin-block-end: var(--space-1);
    text-align: end;
    pointer-events: none;
  }

  .wake-slot :global(.status:has(> span:not(:empty))) {
    padding: var(--space-1) var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }
</style>
