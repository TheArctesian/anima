<script lang="ts">
  import type { Snippet } from 'svelte';
  import { resolve } from '$app/paths';
  import type { DeviceCheck } from '$lib/checks/api';
  import WarningBadge from '$lib/devices/actions/WarningBadge.svelte';
  import WebMenu from '$lib/devices/actions/WebMenu.svelte';
  import { webInterfacesOf } from '$lib/devices/actions/actions';
  import type { DeviceDetail } from '$lib/devices/api';
  import ScreenSharingButton from '$lib/devices/screen-sharing/ScreenSharingButton.svelte';
  import type { ScreenSharingCalls } from '$lib/devices/screen-sharing/setup';
  import StatusBadge from '$lib/devices/status/StatusBadge.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import SplitButton from '$lib/forms/split/SplitButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import DetailBar from '$lib/page/DetailBar.svelte';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import type { SettingsSection } from '../tabs/tabs';
  import { actionWords, connectActions, headerLine, statusReason, type HeaderDevice, type ProtocolAction } from './header';

  interface Props {
    /** Null while the device loads, and when there is none: the bar shows only the back link (D-13, D-14). */
    device: HeaderDevice | null;
    /** The device's checks, for the reason under the bar; `null` until they arrive. */
    checks: readonly DeviceCheck[] | null;
    /** A protocol that is ready: the button is passed so the session tab grows from it (WP-14 H-1, RA-030, MOT-005). */
    onConnect: (protocol: SessionProtocol, button: HTMLElement) => void;
    /** The Web button (WEB-003); it is hidden while no route gives this. */
    onOpenWeb?: (webInterface: WebInterface, button: HTMLElement) => void;
    /** The "Set up screen sharing" button (R-079); it is hidden while no route gives this. `onEnabled` shows the updated device. */
    screenSharing?: { readonly calls: ScreenSharingCalls; readonly onEnabled: (device: DeviceDetail) => void };
    /** Opens the Settings tab, at `section` when given (DEV-022). */
    onSettings: (section: SettingsSection | null) => void;
    /** The device's tabs, the bar's last row. */
    tabs?: Snippet;
  }

  const { device, checks, onConnect, onOpenWeb, screenSharing, onSettings, tabs }: Props = $props();
  const line = $derived(device === null ? null : headerLine(device));
  const connect = $derived(device === null || line === null ? null : connectActions(line, device.name));
  const reason = $derived(device === null ? null : statusReason(device, checks));

  const PROTOCOL_ICONS: Record<SessionProtocol, IconName> = {
    SSH: 'carbon:terminal',
    RDP: 'carbon:remote-connection',
    VNC: 'carbon:screen-map',
  };

  /** Not set up shows a "+" in place of the protocol's icon (DEV-022). */
  const iconOf = (action: ProtocolAction): IconName => (action.mode === 'set-up' ? 'carbon:add' : PROTOCOL_ICONS[action.protocol]);

  /** A protocol action starts a session, or opens Settings at its section and never a session (DEV-022). */
  function run(action: ProtocolAction, button: HTMLElement): void {
    if (action.section === null) onConnect(action.protocol, button);
    else onSettings(action.section);
  }
</script>

{#snippet meta()}
  {#if device !== null}
    <!-- A live region, so a status change is heard as well as seen (D-11); pinned, the reason stays here as a tooltip (D16). -->
    <span class="status" aria-live="polite" title={reason ?? undefined}>
      <StatusBadge status={device.status} deviceId={device.id} />
      <WarningBadge warnings={device.accessWarnings} />
    </span>
    <span class="mono">{device.host}</span>
    {#if line?.ip != null}<span class="mono">{line.ip}</span>{/if}
  {/if}
{/snippet}

{#snippet actions()}
  {#if device !== null && line !== null && connect !== null}
    <!-- One way in (D16): the first protocol that connects, the rest and their set-ups behind the chevron (DEV-022). -->
    <SplitButton
      tone="primary"
      label={actionWords(connect.main)}
      mainLabel={connect.main.label}
      icon={iconOf(connect.main)}
      menuLabel="More ways to connect to {device.name}"
      choices={connect.others.map((action) => ({ id: action.protocol, label: actionWords(action), spoken: action.label, icon: iconOf(action) }))}
      onMain={(button) => { run(connect.main, button); }}
      onChoose={(choice, button) => {
        const chosen = connect.others.find((action) => action.protocol === choice.id);
        if (chosen !== undefined) run(chosen, button);
      }}
    />
    {#if !line.vnc && line.screenSharing && screenSharing !== undefined}
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
    <FormButton tone="ghost" icon="carbon:edit" onclick={() => { onSettings(null); }}>Edit</FormButton>
  {/if}
{/snippet}

{#snippet why()}
  <span>{reason}</span>
  {#if device !== null}
    <a class="review" href="{resolve('/devices')}/{encodeURIComponent(device.id)}?tab=checks">Review<Icon name="carbon:arrow-right" size="sm" /></a>
  {/if}
{/snippet}

<!-- The device's detail bar (D16, DEV-020): Devices / its OS mark and name, its status and address, one split button
     to connect and Edit, the reason it is not up, and the tabs. -->
<DetailBar
  back={{ to: '/devices', label: 'Devices' }}
  icon={line?.osIcon ?? 'carbon:bare-metal-server'}
  title={device?.name ?? null}
  shared={device === null ? undefined : `device-name:${device.id}`}
  meta={device === null ? undefined : meta}
  actions={device === null ? undefined : actions}
  reason={reason === null ? undefined : why}
  {tabs}
/>

<style>
  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .mono {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .review {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--accent);
    text-decoration: none;
    white-space: nowrap;
  }

  .review:hover {
    text-decoration: underline;
    text-underline-offset: var(--space-1);
  }

  .review:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
    border-radius: var(--radius-sm);
  }
</style>
