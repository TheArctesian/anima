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
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';

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
    /** Icon buttons named by their tooltip and label, for the list's fixed connect column (UR-7, D18). */
    compact?: boolean;
    /** A row of the list: no action that cannot work now, so each row keeps one line (row H of the site language plan). */
    row?: boolean;
  }

  const { device, onConnect, onOpenWeb, wake, screenSharing, compact = false, row = false }: Props = $props();
  const ICONS: Record<SessionProtocol, IconName> = { SSH: 'carbon:terminal', RDP: 'carbon:remote-connection', VNC: 'carbon:screen' };
  const web = $derived(webInterfacesOf(device));
  const protocols = $derived(connectProtocols(device));
  // The setup needs the device: a row of the list does not offer it for a device that is down (row H of the site language plan).
  const offersSetup = $derived(screenSharing !== undefined && offersScreenSharing(device) && !(row && device.status === 'DOWN'));
  const offersWeb = $derived(onOpenWeb !== undefined && web.length > 0);
  /**
   * A row of the list has one line: with more than two buttons, each way in shows its icon alone and its name in its
   * tooltip, so a device that goes down and gains "Wake" does not make its row taller (MOT-007).
   */
  const offersWake = $derived(wake !== undefined && canWake(device) && !(row && device.status === 'UP'));
  const tight = $derived(row && protocols.length + (offersSetup ? 1 : 0) + (offersWeb ? 1 : 0) + (offersWake ? 1 : 0) > 2);
</script>

<span class="actions" class:compact class:row>
  {#each protocols as protocol (protocol)}
    <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
    <button
      type="button"
      class="device-action"
      class:quiet={compact && !tight}
      class:icon-only={tight}
      title={tight ? protocol : undefined}
      tabindex="0"
      aria-label={connectLabel(protocol, device.name)}
      onclick={(event) => {
        onConnect(device, protocol, event.currentTarget);
      }}
      {@attach press()}>{#if compact}<Icon name={ICONS[protocol]} size="sm" />{/if}{#if !tight}{protocol}{/if}</button
    >
  {/each}
  {#if screenSharing !== undefined && offersSetup}
    <ScreenSharingButton {device} calls={screenSharing.calls} onReady={screenSharing.onReady} compact />
  {/if}
  {#if onOpenWeb !== undefined && offersWeb}
    <WebMenu
      iconOnly={tight}
      deviceName={device.name}
      interfaces={web}
      onOpen={(webInterface, button) => {
        onOpenWeb(device, webInterface, button);
      }}
    />
  {/if}
  {#if wake !== undefined && offersWake}
    <!-- Up needs no wake: a row of the list leaves the button out, each other place shows it disabled. The slot is one button tall, and the button's status line floats above it: it moves nothing (MOT-007). -->
    <span class="wake-slot"><WakeButton deviceId={device.id} api={wake} disabled={device.status === 'UP'} small={row} /></span>
  {/if}
</span>

<style>
  .actions {
    flex: none;
    display: inline-flex;
    gap: var(--space-2);
  }

  .compact {
    gap: var(--space-1);
  }

  .actions.row {
    flex-wrap: nowrap;
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
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
  }
</style>
