<script lang="ts">
  import WebMenu from '$lib/devices/actions/WebMenu.svelte';
  import { webInterfacesOf } from '$lib/devices/actions/actions';
  import '$lib/devices/actions/actions.css';
  import ScreenSharingButton from '$lib/devices/screen-sharing/ScreenSharingButton.svelte';
  import type { ScreenSharingCalls } from '$lib/devices/screen-sharing/setup';
  import type { DeviceDetail } from '$lib/devices/api';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { press } from '$lib/motion/roles/press';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import type { SettingsSection } from '../tabs/tabs';
  import { protocolButton, type HeaderDevice, type HeaderLine, type ProtocolMode } from './header';

  interface Props {
    device: HeaderDevice;
    line: HeaderLine;
    onConnect: (protocol: SessionProtocol, button: HTMLElement) => void;
    onOpenWeb?: (webInterface: WebInterface, button: HTMLElement) => void;
    screenSharing?: { readonly calls: ScreenSharingCalls; readonly onEnabled: (device: DeviceDetail) => void };
    onSettings: (section: SettingsSection | null) => void;
  }

  const { device, line, onConnect, onOpenWeb, screenSharing, onSettings }: Props = $props();

  const PROTOCOL_ICONS: Record<SessionProtocol, IconName> = {
    SSH: 'nf-md-console',
    RDP: 'nf-md-remote_desktop',
    VNC: 'nf-md-monitor_screenshot',
  };
</script>

{#snippet protocolAction(protocol: SessionProtocol, mode: ProtocolMode)}
  {@const action = protocolButton(protocol, mode, device.name)}
  <!-- Off is dashed with a "+" in place of the protocol icon and opens Settings, never a session (DEV-022). -->
  <button
    type="button"
    class="device-action {mode === 'set-up' ? 'dashed' : 'solid'}"
    tabindex="0"
    aria-label={action.label}
    onclick={(event) => {
      if (action.section === null) onConnect(protocol, event.currentTarget);
      else onSettings(action.section);
    }}
    {@attach press()}
  >
    <Icon name={mode === 'set-up' ? 'nf-md-plus' : PROTOCOL_ICONS[protocol]} size="sm" />{protocol}
  </button>
{/snippet}

<!-- The buttons, in the order of DEV-020: SSH, RDP, VNC or its set-up, Web, then Edit last (D-4). -->
<div class="actions">
  {@render protocolAction('SSH', line.ssh)}
  {@render protocolAction('RDP', line.rdp)}
  {#if line.vnc}
    {@render protocolAction('VNC', 'connect')}
  {:else if screenSharing !== undefined && line.screenSharing}
    <ScreenSharingButton
      {device}
      calls={screenSharing.calls}
      onReady={(updated, button) => {
        onConnect('VNC', button);
        screenSharing.onEnabled(updated);
      }}
    />
  {/if}
  {#if onOpenWeb !== undefined && webInterfacesOf(device).length > 0}
    <WebMenu deviceName={device.name} interfaces={webInterfacesOf(device)} onOpen={onOpenWeb} />
  {/if}
  <button type="button" class="device-action" tabindex="0" onclick={() => { onSettings(null); }} {@attach press()}>
    <Icon name="nf-md-pencil" size="sm" />Edit
  </button>
</div>

<style>
  /* The buttons wrap only in the narrow header, where the header reserves the rows they take (DEV-021, MOT-007). */
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-content: start;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  .solid {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
  }

  .solid:hover,
  .solid:active {
    background: var(--accent-deep);
  }

  /* Not set up: a dashed border and a "+" (DEV-022). */
  .dashed {
    border-style: dashed;
    background: transparent;
  }
</style>
